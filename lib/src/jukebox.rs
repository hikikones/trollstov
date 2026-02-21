use std::{
    collections::{HashSet, VecDeque},
    fs::File,
    path::PathBuf,
    time::Duration,
};

use rodio::decoder::Decoder;

use crate::*;

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<File>, AudioFileReport>>;
type AudioWriteHandle = std::thread::JoinHandle<Result<(TrackId, AudioRating), AudioFileReport>>;

pub struct Jukebox {
    database: Database,
    current: Option<(TrackId, QueueIndex)>,
    queue: PlayQueue,
    state: PlayState,
    audio_decode_handle: Option<(AudioDecodeHandle, TrackId, QueueIndex, PathBuf)>,
    audio_write_handle: Option<AudioWriteHandle>,
    audio_write_queue: VecDeque<(TrackId, AudioRating)>,
    faulty: HashSet<TrackId>,
    events: Vec<JukeboxEvent>,
    device: AudioDevice,
    mpris: Option<MediaControls>,
}

pub enum JukeboxEvent {
    Play(TrackId, PathBuf),
    Stop,
    Rating(TrackId),
    Error(AudioFileReport),
    Focus,
    Quit,
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
    pub fn new(device: AudioDevice, music_dir: PathBuf) -> Self {
        Self {
            database: Database::new(music_dir),
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            audio_decode_handle: None,
            audio_write_handle: None,
            audio_write_queue: VecDeque::new(),
            faulty: HashSet::new(),
            events: Vec::new(),
            device,
            mpris: None,
        }
    }

    pub fn load_music(&mut self) {
        self.database.load();
    }

    pub fn attach_media_controls(&mut self, name: &str) -> Result<(), AudioFileReport> {
        self.mpris = Some(MediaControls::new(name)?);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.database.is_empty()
    }

    pub fn len(&self) -> usize {
        self.database.len()
    }

    pub fn get(&self, id: TrackId) -> Option<&Track> {
        self.database.get(id)
    }

    pub fn get_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.database.get_mut(id)
    }

    pub fn get_id_from_index(&self, i: usize) -> Option<TrackId> {
        self.database.get_id_from_index(i)
    }

    pub fn get_index_from_id(&self, id: TrackId) -> Option<usize> {
        self.database.get_index_from_id(id)
    }

    pub fn get_id_from_queue(&self, i: usize) -> Option<TrackId> {
        self.queue.get(QueueIndex(i))
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.database.iter().map(|(id, track)| (*id, track))
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.database.get_sort()
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.database.sort(sort);
    }

    pub fn search(&mut self, keywords: &str) -> impl Iterator<Item = (TrackId, u16)> {
        self.database.search(keywords)
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

    pub fn play_queue_index(&mut self, index: usize) {
        if let Some(id) = self.queue.set_index(index) {
            self.state = PlayState::Track;
            self.start_play(id, QueueIndex(index));
        }
    }

    pub fn play_track(&mut self, id: TrackId) {
        let (id, index) = self.queue.enqueue_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_play(id, index);
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

        if let Some(mpris) = self.mpris.as_mut() {
            mpris.reset_metadata();
        }
    }

    pub fn play_next(&mut self) {
        if self.database.is_empty() {
            return;
        }

        let mut next = self.queue.current_or_next();

        loop {
            if next == self.current {
                next = self.queue.next();
            }

            match next {
                Some((id, index)) => {
                    if self.faulty.contains(&id) {
                        next = self.queue.next();
                        continue;
                    }

                    self.state = PlayState::Next;
                    self.start_play(id, index);
                    return;
                }
                None => {
                    break;
                }
            }
        }

        // No tracks in the queue, play a random next
        let mut random = TrackId(fastrand::u64(0..self.database.len() as u64));
        if self.current_track_id() == Some(random) {
            // Do it one more time
            random = TrackId(fastrand::u64(0..self.database.len() as u64));
        }
        let (id, index) = self.queue.enqueue(random).next().unwrap();
        self.state = PlayState::Next;
        self.start_play(id, index);
    }

    pub fn play_previous(&mut self) {
        while let Some((id, index)) = self.queue.previous() {
            if self.faulty.contains(&id) {
                continue;
            }

            self.state = PlayState::Previous;
            self.start_play(id, index);
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
        self.audio_write_queue.push_back((id, rating));
    }

    fn start_play(&mut self, id: TrackId, index: QueueIndex) {
        let Some(track) = self.database.get(id) else {
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
        self.audio_decode_handle = Some((handle, id, index, track.path().to_path_buf()));
    }

    fn write_rating(&mut self, id: TrackId, rating: AudioRating) -> Option<AudioWriteHandle> {
        let Some(track) = self.database.get(id) else {
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

    pub fn update(&mut self, mut on_event: impl FnMut(JukeboxEvent)) {
        self.database.update(|error| {
            self.events.push(JukeboxEvent::Error(error));
        });

        // Poll thread handle for audio decoding
        if let Some((handle, _, _, _)) = self.audio_decode_handle.as_ref() {
            if handle.is_finished() {
                let (handle, id, index, path) = self.audio_decode_handle.take().unwrap();
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
                        self.events.push(JukeboxEvent::Play(id, path));

                        // Update metadata for media control
                        if let Some(mpris) = self.mpris.as_mut()
                            && let Some(track) = self.database.get(id)
                        {
                            mpris.set_metadata(track.title(), track.artist());
                        }
                    }
                    // Failed to decode audio
                    Err(err) => {
                        self.faulty.insert(id);
                        self.events.push(JukeboxEvent::Error(err));
                        match self.state {
                            PlayState::Play | PlayState::Next => {
                                self.play_next();
                            }
                            PlayState::Previous => {
                                self.play_previous();
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
                            if let Some(track) = self.database.get_mut(id) {
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
                    .and_then(|(id, rating)| self.write_rating(id, rating));
            }
        }

        // Check for media control events
        if let Some(event) = self.mpris.as_ref().and_then(|mpris| mpris.try_recv()) {
            match event {
                MediaEvent::Play => self.play(),
                MediaEvent::Pause => self.pause(),
                MediaEvent::Toggle => self.pause_or_play(),
                MediaEvent::Next => self.play_next(),
                MediaEvent::Previous => self.play_previous(),
                MediaEvent::Stop => self.stop(),
                MediaEvent::Raise => {
                    self.events.push(JukeboxEvent::Focus);
                }
                MediaEvent::Quit => {
                    self.events.push(JukeboxEvent::Quit);
                }
            }
        }
        // Play next when empty and idle
        else if self.device.is_empty() && !self.device.is_paused() {
            match self.state {
                PlayState::Play => {
                    self.play_next();
                }
                PlayState::Next | PlayState::Previous | PlayState::Track => {
                    self.state = PlayState::Play;
                }
                _ => {}
            }
        }

        for event in self.events.drain(..) {
            on_event(event);
        }
    }

    pub fn shutdown(mut self) {
        // Gracefully shutdown by waiting for thread to finish writing tag
        if let Some(handle) = self.audio_write_handle.take() {
            let _ = handle.join().unwrap();
        }
    }
}
