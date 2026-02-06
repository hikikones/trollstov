use std::{collections::HashSet, fs::File, io::BufReader, path::Path, time::Duration};

use rodio::decoder::Decoder;

use crate::*;

type AudioDecodeHandle = std::thread::JoinHandle<Result<Decoder<BufReader<File>>, AudioFileReport>>;
type AudioWriteHandle =
    std::thread::JoinHandle<Result<(TrackId, Option<AudioRating>), AudioFileReport>>;

pub struct Jukebox {
    database: Database,
    current: Option<(TrackId, QueueIndex)>,
    queue: PlayQueue,
    state: PlayState,
    audio_decode_handle: Option<(TrackId, QueueIndex, AudioDecodeHandle)>,
    audio_write_handles: Vec<AudioWriteHandle>,
    faulty: HashSet<TrackId>,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
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
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        Ok(Self {
            database: Database::new(dir.as_ref().to_path_buf()),
            current: None,
            queue: PlayQueue::new(),
            state: PlayState::Stop,
            audio_decode_handle: None,
            audio_write_handles: Vec::new(),
            faulty: HashSet::new(),
            sink,
            _stream: stream,
        })
    }

    pub fn load(&mut self) {
        self.database.load();
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

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.database.iter()
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.database.get_sort()
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.database.sort(sort);
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
        self.sink.get_pos()
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

    pub fn enqueue(&mut self, id: TrackId) {
        self.queue.enqueue(id);
    }

    pub fn enqueue_next(&mut self, id: TrackId) {
        self.queue.enqueue_next(id);
    }

    pub fn play_queue_index(&mut self, index: usize) {
        if let Some(id) = self.queue.set_index(QueueIndex(index)) {
            self.state = PlayState::Play;
            self.start_play(id, QueueIndex(index));
        }
    }

    pub fn play_track(&mut self, id: TrackId) {
        // TODO: If new track is same as current, simply rewind.
        let (id, index) = self.queue.enqueue_next(id).next().unwrap();
        self.state = PlayState::Track;
        self.start_play(id, index);
    }

    pub fn play(&mut self) {
        self.state = PlayState::Play;
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.state = PlayState::Pause;
        self.sink.pause();
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        // TODO: Should stop also clear queue and history?
        self.sink.clear();
        self.current = None;
        self.state = PlayState::Stop;
        self.audio_decode_handle = None;
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

        // No tracks in the queue, enqueue a random
        let random = fastrand::u64(0..self.database.len() as u64);
        let (id, index) = self.queue.enqueue(TrackId(random)).next().unwrap();
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
    }

    pub fn fast_forward_by(&mut self, duration: Duration) {
        if self.sink.empty() {
            return;
        }

        let _ = self.sink.try_seek(self.sink.get_pos() + duration);
    }

    pub fn set_rating(&mut self, id: TrackId, rating: AudioRating) {
        let handle = self.write_rating(id, rating);
        self.audio_write_handles.push(handle);
    }

    fn start_play(&mut self, id: TrackId, index: QueueIndex) {
        self.audio_decode_handle = Some((id, index, self.decode_audio(id)));
    }

    fn decode_audio(&mut self, id: TrackId) -> AudioDecodeHandle {
        let track = self.database.get(id).unwrap();
        let path = track.path().to_path_buf();

        std::thread::spawn(move || {
            let file = File::open(&path).map_err(|err| {
                AudioFileReport::new(format!(
                    "Could not open audio file {} due to {}",
                    path.display(),
                    err
                ))
            })?;
            let buffer = BufReader::new(file);
            let source = Decoder::new(buffer).map_err(|err| {
                AudioFileReport::new(format!(
                    "Could not decode audio file {} due to {}",
                    path.display(),
                    err
                ))
            })?;
            Ok(source)
        })
    }

    fn write_rating(&mut self, id: TrackId, rating: AudioRating) -> AudioWriteHandle {
        let track = self.database.get(id).unwrap();
        let path = track.path().to_path_buf();
        let extension = track.extension();
        let current_rating = track.rating();

        let new_rating = match current_rating {
            Some(current_rating) => {
                if current_rating != rating {
                    // Replace rating when they differ
                    Some(rating)
                } else {
                    // Remove rating when they are the same
                    None
                }
            }
            None => {
                // Insert new rating
                Some(rating)
            }
        };

        std::thread::spawn(move || {
            let mut audio_file = AudioFile::read_from(path, extension)?;
            audio_file.write_rating(new_rating)?;
            Ok((id, new_rating))
        })
    }

    pub fn update(&mut self, mut on_error: impl FnMut(AudioFileReport)) -> bool {
        self.database.update(&mut on_error);

        let mut render = false;

        // Poll thread handle for audio decoding
        if let Some((_, _, handle)) = self.audio_decode_handle.as_ref() {
            if handle.is_finished() {
                render = true;
                let (id, index, handle) = self.audio_decode_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok(decoded_audio) => {
                        self.sink.clear();
                        self.sink.append(decoded_audio);
                        if self.state != PlayState::Pause {
                            self.state = PlayState::Play;
                            self.sink.play();
                        }
                        self.current = Some((id, index));
                    }
                    Err(err) => {
                        on_error(err);
                        self.faulty.insert(id);
                        match self.state {
                            PlayState::Play | PlayState::Next => {
                                self.play_next();
                            }
                            PlayState::Previous => {
                                self.play_previous();
                            }
                            PlayState::Track => {
                                self.state = PlayState::Play;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Poll thread handles for finished tag writing
        // TODO: Write handles should be polled one by one.
        // Right now, multiple threads _could_ be writing to the same file.
        for _ in 0..self.audio_write_handles.len() {
            let handle = self.audio_write_handles.pop().unwrap();

            if !handle.is_finished() {
                self.audio_write_handles.push(handle);
                continue;
            }

            match handle.join().unwrap() {
                Ok((id, new_rating)) => {
                    let track = self.get_mut(id).unwrap();
                    track.set_rating(new_rating);
                    render = true;
                }
                Err(err) => {
                    on_error(err);
                }
            }
        }

        // Play next when empty and idle
        let is_finished = self.sink.empty() && !self.sink.is_paused();
        if is_finished {
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

        render
    }

    pub fn shutdown(self) {
        // Gracefully shutdown by waiting for threads to finish writing tag
        for handle in self.audio_write_handles {
            let _ = handle.join().unwrap();
        }
    }
}
