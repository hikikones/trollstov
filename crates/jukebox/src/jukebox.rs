use std::{collections::HashSet, fs::File, time::Duration};

use rodio::decoder::Decoder;

use audio::*;
use database::*;

use crate::{AudioPlayer, PlayQueue};

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<File>, AudioFileReport>>;

pub struct Jukebox {
    current: Option<(TrackId, usize)>,
    queue: PlayQueue,
    state: PlayState,
    skip: AudioRating,
    decode_handle: Option<(AudioDecodeHandle, TrackId, usize)>,
    events: Vec<JukeboxEvent>,
    faulty: HashSet<TrackId>,
    player: AudioPlayer,
}

pub enum JukeboxEvent {
    Play(Option<TrackId>),
    Pause,
    Stop,
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
    pub fn new(player: AudioPlayer) -> Self {
        Self {
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            skip: AudioRating::default(),
            decode_handle: None,
            events: Vec::new(),
            faulty: HashSet::new(),
            player,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn is_faulty(&self, id: TrackId) -> bool {
        self.faulty.contains(&id)
    }

    pub const fn len(&self) -> usize {
        self.queue.len()
    }

    pub const fn queue(&self) -> usize {
        self.queue.queue_len()
    }

    pub const fn history(&self) -> usize {
        self.queue.history_len()
    }

    pub fn get(&self, index: usize) -> Option<TrackId> {
        self.queue.get(index)
    }

    pub const fn current_track(&self) -> Option<(TrackId, usize)> {
        self.current
    }

    pub fn current_track_id(&self) -> Option<TrackId> {
        self.current.map(|(id, _)| id)
    }

    pub fn current_queue_index(&self) -> Option<usize> {
        self.current.map(|(_, index)| index)
    }

    pub fn current_track_pos(&self) -> Duration {
        self.player.position()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, usize)> {
        self.queue.iter()
    }

    pub fn shuffle(&mut self) {
        let start = match self.current_queue_index() {
            Some(index) => index + 1,
            None => 0,
        };
        self.queue.shuffle(start);
    }

    pub fn move_up(&mut self, i: usize) -> bool {
        let success = self.queue.move_up(i);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_up_range(&mut self, start: usize, end: usize) -> bool {
        let success = self.queue.move_up_range(start, end);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_down(&mut self, i: usize) -> bool {
        let success = self.queue.move_down(i);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn move_down_range(&mut self, start: usize, end: usize) -> bool {
        let success = self.queue.move_down_range(start, end);
        self.current = self.current.and_then(|_| self.queue.current());
        success
    }

    pub fn remove(&mut self, index: usize) -> bool {
        let keep_current = self.current.is_some();
        let removal = self.queue.remove(index, keep_current).is_some();

        if keep_current {
            self.current = self.queue.current();
        }

        removal
    }

    pub fn remove_range(&mut self, start: usize, end: usize) -> bool {
        let keep_current = self.current.is_some();
        let removal = self.queue.remove_range(start, end, keep_current);

        if keep_current {
            self.current = self.queue.current();
        }

        removal
    }

    pub fn clear(&mut self) {
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
        self.player.volume()
    }

    pub fn set_volume(&mut self, value: f32) {
        self.player.set_volume(value);
    }

    pub fn play_index(&mut self, index: usize, db: &Database) {
        if let Some(id) = self.queue.set_index(index) {
            self.state = PlayState::Track;
            self.start_decode(id, index, db);
        }
    }

    pub fn play_id(&mut self, id: TrackId, db: &Database) {
        let (id, index) = self.queue.enqueue_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_decode(id, index, db);
    }

    pub fn play(&mut self) {
        if self.state == PlayState::Play {
            return;
        }

        self.state = PlayState::Play;
        self.player.play();
        self.events.push(JukeboxEvent::Play(None));
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Pause {
            return;
        }

        self.state = PlayState::Pause;
        self.player.pause();
        self.events.push(JukeboxEvent::Pause);
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

        self.state = PlayState::Stop;
        self.current = None;
        self.decode_handle = None;
        self.player.clear();
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
                    if self.is_faulty(id) || self.should_skip(id, db) {
                        next = self.queue.next();
                        continue;
                    }

                    self.state = PlayState::Next;
                    self.start_decode(id, index, db);
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
            rand = db.random_id();
            let is_current = current.map(|id| id == rand).unwrap_or(false);
            if is_current || self.is_faulty(rand) || self.should_skip(rand, db) {
                tries -= 1;
                continue;
            }
            break;
        }

        let (id, index) = self.queue.enqueue(rand).next().unwrap();
        self.state = PlayState::Next;
        self.start_decode(id, index, db);
    }

    pub fn play_previous(&mut self, db: &Database) {
        while let Some((id, index)) = self.queue.previous() {
            if self.is_faulty(id) || self.should_skip(id, db) {
                continue;
            }

            self.state = PlayState::Previous;
            self.start_decode(id, index, db);
            return;
        }

        // No valid previous found, update queue index
        self.sync_queue_index();
    }

    pub fn fast_forward_by(&mut self, duration: Duration) {
        if self.player.is_empty() {
            return;
        }

        self.player.seek(self.player.position() + duration);
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

    fn start_decode(&mut self, id: TrackId, index: usize, db: &Database) {
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
        self.decode_handle = Some((handle, id, index));
    }

    fn sync_queue_index(&mut self) {
        match self.current {
            Some((_, index)) => {
                self.queue.set_index(index);
            }
            None => {
                self.queue.reset();
            }
        }
    }

    pub fn update(&mut self, db: &Database, mut on_event: impl FnMut(JukeboxEvent)) {
        // Poll thread handle for audio decoding
        if let Some((handle, _, _)) = self.decode_handle.as_ref() {
            if handle.is_finished() {
                let (handle, id, index) = self.decode_handle.take().unwrap();
                match handle.join().unwrap() {
                    // Play successfully decoded audio and update state
                    Ok(decoded_audio) => {
                        self.player.clear();
                        self.player.append(decoded_audio);
                        if self.state != PlayState::Pause {
                            self.state = PlayState::Play;
                            self.player.play();
                        }
                        self.current = Some((id, index));
                        self.events.push(JukeboxEvent::Play(Some(id)));
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

        // Play next when empty and idle
        if self.player.is_empty() && !self.player.is_paused() {
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

        // Drain events
        for event in self.events.drain(..) {
            on_event(event);
        }
    }
}
