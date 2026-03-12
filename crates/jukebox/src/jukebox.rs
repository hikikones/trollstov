use std::{collections::HashSet, fs::File, time::Duration};

use rodio::decoder::Decoder;

use crate::*;

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<File>, AudioFileReport>>;

pub struct Jukebox {
    current: Option<(TrackId, usize)>,
    queue: PlayQueue,
    state: PlayState,
    skip: AudioRating,
    decode_handle: Option<(AudioDecodeHandle, TrackId, usize)>,
    events: Vec<JukeboxEvent>,
    faulty: HashSet<TrackId>,
    device: AudioDevice,
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
    pub fn new(device: AudioDevice) -> Self {
        Self {
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            skip: AudioRating::default(),
            decode_handle: None,
            events: Vec::new(),
            faulty: HashSet::new(),
            device,
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
        self.device.position()
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
        self.device.volume()
    }

    pub fn set_volume(&mut self, value: f32) {
        self.device.set_volume(value);
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
        self.device.play();
        self.events.push(JukeboxEvent::Play(None));
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Pause {
            return;
        }

        self.state = PlayState::Pause;
        self.device.pause();
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
                        self.device.clear();
                        self.device.append(decoded_audio);
                        if self.state != PlayState::Pause {
                            self.state = PlayState::Play;
                            self.device.play();
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

        // Drain events
        for event in self.events.drain(..) {
            on_event(event);
        }
    }
}

// TODO: Max length? Drain from history.
// TODO: Move up/down.

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

    fn get(&self, index: usize) -> Option<TrackId> {
        self.list.get(index).copied()
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

    fn current(&self) -> Option<(TrackId, usize)> {
        self.index
            .and_then(|i| self.list.get(i).copied().map(|id| (id, i)))
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, usize)> {
        self.list.iter().enumerate().map(|(i, id)| (*id, i))
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

    fn current_or_next(&mut self) -> Option<(TrackId, usize)> {
        self.current().or_else(|| self.next())
    }

    fn next(&mut self) -> Option<(TrackId, usize)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                let max_index = self.len().saturating_sub(1);
                index = usize::min(index + 1, max_index);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied().map(|id| (id, index))
                } else {
                    None
                }
            }
            None => {
                if self.list.is_empty() {
                    None
                } else {
                    self.index = Some(0);
                    self.list.first().copied().map(|id| (id, 0))
                }
            }
        }
    }

    fn previous(&mut self) -> Option<(TrackId, usize)> {
        match self.index {
            Some(mut index) => {
                let old_index = index;
                index = index.saturating_sub(1);

                if index != old_index {
                    self.index = Some(index);
                    self.list.get(index).copied().map(|id| (id, index))
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

    fn remove(&mut self, index: usize, keep_current: bool) -> Option<TrackId> {
        if index >= self.len() {
            return None;
        }

        let Some(current) = self.index else {
            return Some(self.list.remove(index));
        };

        if index == current && keep_current {
            return None;
        }

        let id = self.list.remove(index);
        self.index = self.index.and_then(|current| {
            if self.list.is_empty() {
                None
            } else if index < current {
                Some(current - 1)
            } else {
                Some(current.min(self.list.len().saturating_sub(1)))
            }
        });
        Some(id)
    }

    fn remove_range(&mut self, start: usize, end: usize, keep_current: bool) -> bool {
        let end = end.min(self.list.len().saturating_sub(1));

        if start > end {
            return false;
        }

        let Some(current) = self.index else {
            self.list.drain(start..=end);
            return true;
        };

        let id = self.list[current];

        let mut offset = 0;
        for index in self
            .list
            .drain(start..=end)
            .enumerate()
            .map(|(i, _)| start + i)
        {
            if index < current {
                offset += 1;
            }
        }

        let contains_current = current >= start && current <= end;
        let keep_current = contains_current && keep_current;
        if self.list.is_empty() {
            if keep_current {
                self.list.push(id);
                self.index = Some(0);
            } else {
                self.index = None;
            }
        } else {
            if keep_current {
                let index = (current - offset).min(self.list.len());
                self.list.insert(index, id);
                self.index = Some(index);
            } else {
                let index = (current - offset).min(self.list.len().saturating_sub(1));
                self.index = Some(index);
            }
        }

        true
    }

    const fn reset(&mut self) {
        self.index = None;
    }

    fn clear(&mut self) {
        self.list.clear();
        self.index = None;
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
        assert_eq!(queue.next(), Some((TrackId(0), 0)));
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.next(), Some((TrackId(1), 1)));
        assert_eq!(queue.current(), Some((TrackId(1), 1)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), TRACKS_LEN - 2);
        assert_eq!(queue.history_len(), 1);

        assert_eq!(queue.next(), None);
        assert_eq!(queue.current(), Some((TrackId(1), 1)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 0);
        assert_eq!(queue.history_len(), 1);

        // Previous
        assert_eq!(queue.previous(), Some((TrackId(0), 0)));
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);

        assert_eq!(queue.previous(), None);
        assert_eq!(queue.current(), Some((TrackId(0), 0)));
        assert_eq!(queue.len(), TRACKS_LEN);
        assert_eq!(queue.queue_len(), 1);
        assert_eq!(queue.history_len(), 0);
    }
}
