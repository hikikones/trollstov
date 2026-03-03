use std::{
    collections::{HashSet, VecDeque},
    fs::File,
    time::Duration,
};

use rodio::decoder::Decoder;

use crate::*;

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<File>, AudioFileReport>>;
type AudioWriteHandle = std::thread::JoinHandle<Result<(TrackId, AudioRating), AudioFileReport>>;

pub struct Jukebox {
    // database: Database,
    current: Option<(TrackId, QueueIndex)>,
    queue: PlayQueue,
    state: PlayState,
    skip: AudioRating,
    audio_decode_handle: Option<(AudioDecodeHandle, TrackId, QueueIndex)>,
    audio_write_handle: Option<AudioWriteHandle>,
    audio_write_queue: VecDeque<(TrackId, AudioRating)>,
    faulty: HashSet<TrackId>,
    events: Vec<JukeboxEvent>,
    device: AudioDevice,
}

pub enum JukeboxEvent {
    Play(TrackId),
    Stop,
    Rating(TrackId),
    Error(AudioFileReport),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Track,
}

impl Jukebox {
    pub fn new(device: AudioDevice) -> Self {
        Self {
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            skip: AudioRating::default(),
            audio_decode_handle: None,
            audio_write_handle: None,
            audio_write_queue: VecDeque::new(),
            faulty: HashSet::new(),
            events: Vec::new(),
            device,
        }
    }

    pub fn is_faulty(&self, id: TrackId) -> bool {
        self.faulty.contains(&id)
    }

    pub fn get_id_from_queue(&self, i: usize) -> Option<TrackId> {
        self.queue.get(QueueIndex(i))
    }

    pub const fn current_track(&self) -> Option<(TrackId, QueueIndex)> {
        self.current
    }

    pub fn current_track_id(&self) -> Option<TrackId> {
        self.current.map(|(id, _)| id)
    }

    pub fn current_queue_index(&self) -> Option<QueueIndex> {
        self.current.map(|(_, index)| index)
    }

    pub fn current_track_pos(&self) -> Duration {
        self.device.position()
    }

    pub const fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub const fn queue_total(&self) -> usize {
        self.queue.len()
    }

    pub const fn queue_len(&self) -> usize {
        self.queue.queue_len()
    }

    pub const fn history_len(&self) -> usize {
        self.queue.history_len()
    }

    pub fn queue_iter(&self) -> impl ExactSizeIterator<Item = (TrackId, QueueIndex)> {
        self.queue.iter()
    }

    pub fn queue_shuffle(&mut self) {
        let start = match self.current_queue_index() {
            Some(index) => index.0 + 1,
            None => 0,
        };
        self.queue.shuffle(start);
    }

    pub fn queue_clear(&mut self) {
        self.queue.clear();

        if let Some((id, _)) = self.current.take() {
            self.current = self.queue.enqueue(id).next();
        }
    }

    pub fn enqueue(&mut self, id: TrackId) {
        self.queue.enqueue(id);
    }

    pub fn enqueue_next(&mut self, id: TrackId) {
        self.queue.enqueue_next(id);
    }

    pub fn volume(&self) -> f32 {
        self.device.volume()
    }

    pub fn set_volume(&mut self, value: f32) {
        self.device.set_volume(value);
    }

    pub fn play_queue_index(&mut self, index: usize, db: &Database) {
        if let Some(id) = self.queue.set_index(index) {
            self.state = PlayState::Track;
            self.start_play(id, QueueIndex(index), db);
        }
    }

    pub fn play_track(&mut self, id: TrackId, db: &Database) {
        let (id, index) = self.queue.enqueue_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_play(id, index, db);
    }

    pub fn play(&mut self) {
        self.state = PlayState::Play;
        self.device.play();
    }

    pub fn pause(&mut self) {
        self.state = PlayState::Pause;
        self.device.pause();
    }

    pub fn pause_or_play(&mut self) {
        match self.state {
            PlayState::Pause | PlayState::Stop => self.play(),
            _ => self.pause(),
        }
    }

    pub fn stop(&mut self) {
        if self.state == PlayState::Stop {
            return;
        }

        self.current = None;
        self.audio_decode_handle = None;
        self.state = PlayState::Stop;
        self.device.clear();
        self.events.push(JukeboxEvent::Stop);
    }

    pub fn play_next(&mut self, db: &Database) {
        if db.is_empty() {
            return;
        }

        let mut next = self.queue.current_or_next();

        loop {
            if next == self.current {
                next = self.queue.next();
            }

            match next {
                Some((id, index)) => {
                    if self.faulty.contains(&id) || self.should_skip(id, db) {
                        next = self.queue.next();
                        continue;
                    }

                    self.state = PlayState::Next;
                    self.start_play(id, index, db);
                    return;
                }
                None => {
                    break;
                }
            }
        }

        // No tracks in the queue, play a random next
        let current = self.current_track_id();
        let mut tries = 5;
        let mut rand = TrackId(0);
        while tries > 0 {
            rand = TrackId(fastrand::u64(0..db.len() as u64));
            let is_current = current.map(|id| id == rand).unwrap_or(false);
            if is_current || self.faulty.contains(&rand) || self.should_skip(rand, db) {
                tries -= 1;
                continue;
            }
            break;
        }

        let (id, index) = self.queue.enqueue(rand).next().unwrap();
        self.state = PlayState::Next;
        self.start_play(id, index, db);
    }

    pub fn play_previous(&mut self, db: &Database) {
        while let Some((id, index)) = self.queue.previous() {
            if self.faulty.contains(&id) || self.should_skip(id, db) {
                continue;
            }

            self.state = PlayState::Previous;
            self.start_play(id, index, db);
            return;
        }

        // No valid previous found, update queue index
        self.sync_queue_index();
    }

    pub fn fast_forward_by(&mut self, duration: Duration) {
        if self.device.is_empty() {
            return;
        }

        self.device.seek(self.device.position() + duration);
    }

    pub fn set_rating(&mut self, id: TrackId, rating: AudioRating) {
        self.audio_write_queue.push_back((id, rating)); // TODO: move writing to db
    }

    pub const fn set_skip(&mut self, rating: AudioRating) {
        self.skip = rating;
    }

    fn should_skip(&self, id: TrackId, db: &Database) -> bool {
        db.get(id)
            .map(|track| match track.rating() {
                AudioRating::None => false,
                _ => track.rating().as_u8() <= self.skip.as_u8(),
            })
            .unwrap_or(true)
    }

    fn start_play(&mut self, id: TrackId, index: QueueIndex, db: &Database) {
        let Some(track) = db.get(id) else {
            return;
        };

        let path = track.path().to_path_buf();
        let extension = track.extension();
        let handle = std::thread::spawn(move || {
            let file = File::open(&path).map_err(|err| {
                AudioFileReport::new(format!(
                    "Failed to open \"{}\" due to {}",
                    path.display(),
                    err
                ))
            })?;
            let source = Decoder::builder()
                .with_data(file)
                .with_hint(extension.as_lower_case())
                .with_gapless(false)
                .build()
                .map_err(|err| {
                    AudioFileReport::new(format!(
                        "Failed to decode \"{}\" due to {}",
                        path.display(),
                        err
                    ))
                })?;
            Ok(source)
        });
        self.audio_decode_handle = Some((handle, id, index));
    }

    fn write_rating(
        &mut self,
        id: TrackId,
        rating: AudioRating,
        db: &Database,
    ) -> Option<AudioWriteHandle> {
        let Some(track) = db.get(id) else {
            return None;
        };

        if track.rating() == rating {
            return None;
        }

        let path = track.path().to_path_buf();
        let extension = track.extension();
        let handle = std::thread::spawn(move || {
            let mut audio_file = AudioFile::read_from(path, extension)?;
            audio_file.write_rating(rating)?;
            Ok((id, rating))
        });
        Some(handle)
    }

    fn sync_queue_index(&mut self) {
        match self.current {
            Some((_, index)) => {
                self.queue.set_index(index.0);
            }
            None => {
                self.queue.reset();
            }
        }
    }

    pub fn update(&mut self, db: &mut Database) {
        db.update(|error| {
            self.events.push(JukeboxEvent::Error(error));
        });

        // Poll thread handle for audio decoding
        if let Some((handle, _, _)) = self.audio_decode_handle.as_ref() {
            if handle.is_finished() {
                let (handle, id, index) = self.audio_decode_handle.take().unwrap();
                match handle.join().unwrap() {
                    // Play successfully decoded audio and update state
                    Ok(decoded_audio) => {
                        self.device.clear();
                        self.device.append(decoded_audio);
                        if self.state != PlayState::Pause {
                            self.state = PlayState::Play;
                            self.device.play();
                        }
                        self.current = Some((id, index));
                        self.events.push(JukeboxEvent::Play(id));
                    }
                    // Failed to decode audio
                    Err(err) => {
                        self.faulty.insert(id);
                        self.events.push(JukeboxEvent::Error(err));
                        match self.state {
                            PlayState::Play | PlayState::Next => {
                                self.play_next(db);
                            }
                            PlayState::Previous => {
                                self.play_previous(db);
                            }
                            PlayState::Track => {
                                self.state = PlayState::Play;
                                self.sync_queue_index();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Poll thread handle for finished tag writing
        match self.audio_write_handle.as_ref() {
            Some(handle) => {
                if handle.is_finished() {
                    let handle = self.audio_write_handle.take().unwrap();
                    match handle.join().unwrap() {
                        Ok((id, new_rating)) => {
                            if let Some(track) = db.get_mut(id) {
                                track.set_rating(new_rating);
                                self.events.push(JukeboxEvent::Rating(id));
                            }
                        }
                        Err(err) => {
                            self.events.push(JukeboxEvent::Error(err));
                        }
                    }
                }
            }
            None => {
                self.audio_write_handle = self
                    .audio_write_queue
                    .pop_front()
                    .and_then(|(id, rating)| self.write_rating(id, rating, db));
            }
        }

        // Play next when empty and idle
        if self.device.is_empty() && !self.device.is_paused() {
            match self.state {
                PlayState::Play => {
                    self.play_next(db);
                }
                PlayState::Next | PlayState::Previous | PlayState::Track => {
                    self.state = PlayState::Play;
                }
                _ => {}
            }
        }
    }

    pub fn events(&self) -> impl Iterator<Item = &JukeboxEvent> {
        self.events.iter()
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn shutdown(mut self) {
        // Gracefully shutdown by waiting for thread to finish writing tag
        if let Some(handle) = self.audio_write_handle.take() {
            let _ = handle.join().unwrap();
        }
    }
}
