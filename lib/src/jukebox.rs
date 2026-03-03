use std::{collections::HashSet, fs::File, time::Duration};

use rodio::decoder::Decoder;

use crate::*;

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<File>, AudioFileReport>>;

pub struct Jukebox {
    current: Option<(TrackId, QueueIndex)>,
    queue: PlayQueue,
    state: PlayState,
    skip: AudioRating,
    audio_decode_handle: Option<(AudioDecodeHandle, TrackId, QueueIndex)>,
    faulty: HashSet<TrackId>,
    device: AudioDevice,
}

pub enum JukeboxEvent {
    Play(TrackId),
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
            faulty: HashSet::new(),
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

    pub fn stop(&mut self) -> bool {
        if self.state == PlayState::Stop {
            return false;
        }

        self.current = None;
        self.audio_decode_handle = None;
        self.state = PlayState::Stop;
        self.device.clear();
        true
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

    pub fn update(&mut self, db: &Database, mut on_event: impl FnMut(JukeboxEvent)) {
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
                        on_event(JukeboxEvent::Play(id));
                    }
                    // Failed to decode audio
                    Err(err) => {
                        self.faulty.insert(id);
                        on_event(JukeboxEvent::Error(err));
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
}

// TODO: Max length? Drain from history.
// TODO: Move up/down.
// TODO: Remove.

struct PlayQueue {
    list: Vec<TrackId>,
    index: Option<usize>,
}

impl PlayQueue {
    const fn new() -> Self {
        Self {
            list: Vec::new(),
            index: None,
        }
    }

    const fn len(&self) -> usize {
        self.list.len()
    }

    const fn queue_len(&self) -> usize {
        match self.index {
            Some(index) => (self.list.len() - index).saturating_sub(1),
            None => self.list.len(),
        }
    }

    const fn history_len(&self) -> usize {
        match self.index {
            Some(index) => index,
            None => 0,
        }
    }

    const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn get(&self, index: QueueIndex) -> Option<TrackId> {
        self.list.get(index.0).copied()
    }

    fn set_index(&mut self, index: usize) -> Option<TrackId> {
        match self.list.get(index).copied() {
            Some(id) => {
                self.index = Some(index);
                Some(id)
            }
            None => None,
        }
    }

    fn current(&self) -> Option<(TrackId, QueueIndex)> {
        self.index
            .and_then(|i| self.list.get(i).copied().map(|id| (id, QueueIndex(i))))
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, QueueIndex)> {
        self.list
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, QueueIndex(i)))
    }

    fn enqueue(&mut self, id: TrackId) -> &mut Self {
        self.list.push(id);
        self
    }

    fn enqueue_next(&mut self, id: TrackId) -> &mut Self {
        let insert_index = self.index.map(|i| i + 1).unwrap_or_default();
        self.list.insert(insert_index, id);
        self
    }

    fn current_or_next(&mut self) -> Option<(TrackId, QueueIndex)> {
        self.current().or_else(|| self.next())
    }

    fn next(&mut self) -> Option<(TrackId, QueueIndex)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                let max_index = self.len().saturating_sub(1);
                index = usize::min(index + 1, max_index);

                if index != old_index {
                    self.index = Some(index);
                    self.list
                        .get(index)
                        .copied()
                        .map(|id| (id, QueueIndex(index)))
                } else {
                    None
                }
            }
            None => {
                if self.list.is_empty() {
                    None
                } else {
                    self.index = Some(0);
                    self.list.first().copied().map(|id| (id, QueueIndex(0)))
                }
            }
        }
    }

    fn previous(&mut self) -> Option<(TrackId, QueueIndex)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                index = index.saturating_sub(1);

                if index != old_index {
                    self.index = Some(index);
                    self.list
                        .get(index)
                        .copied()
                        .map(|id| (id, QueueIndex(index)))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn shuffle(&mut self, start: usize) {
        let end = self.list.len();
        if start >= end {
            return;
        }

        for i in start..end {
            let random = fastrand::usize(start..end);
            self.list.swap(i, random);
        }
    }

    const fn reset(&mut self) {
        self.index = None;
    }

    fn clear(&mut self) {
        self.list.clear();
        self.index = None;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueIndex(usize);

impl QueueIndex {
    pub const fn raw(self) -> usize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_queue() {
        const TRACKS_LEN: usize = 2;
        let mut queue = PlayQueue::new();

        for i in 0..TRACKS_LEN {
            queue.enqueue(TrackId(i as u64));
        }

        assert_eq!(queue.current(), None);
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN);
        assert_eq!(queue.history_len(), 0);

        // Next
        assert_eq!(queue.next(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.next(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.current(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 2);
        assert_eq!(queue.history_len(), 1);

        assert_eq!(queue.next(), None);
        assert_eq!(queue.current(), Some((TrackId(1), QueueIndex(1))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(queue.history_len(), 1);

        // Previous
        assert_eq!(queue.previous(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.previous(), None);
        assert_eq!(queue.current(), Some((TrackId(0), QueueIndex(0))));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);
    }
}
