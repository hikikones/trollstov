use std::{
    cmp::Ordering,
    collections::VecDeque,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use indexmap::IndexMap;
use rodio::decoder::Decoder;

use crate::{
    audio::*,
    events::{AppEvent, EventSender},
    pages::Log,
    utils,
};

type AudioFileReceiver = mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileReport>>;
type AudioDecodeHandle =
    std::thread::JoinHandle<Result<(TrackId, Decoder<BufReader<File>>), AudioFileReport>>;
type AudioWriteHandle =
    std::thread::JoinHandle<Result<(TrackId, Option<AudioRating>), AudioFileReport>>;

pub struct Jukebox {
    music_dir: PathBuf,
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    current: Option<TrackId>,
    current_actual: Option<TrackId>,
    stopped: Option<TrackId>,
    queue: PlayQueue,
    state: JukeboxState,
    audio_file_receiver: Option<AudioFileReceiver>,
    audio_decode_handle: Option<AudioDecodeHandle>,
    audio_write_handles: Vec<AudioWriteHandle>,
    events: EventSender,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JukeboxState {
    Track,
    Play,
    Pause,
    Stop,
    Next,
    Prev,
}

impl Jukebox {
    pub fn new(dir: impl AsRef<Path>, events: EventSender) -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        Ok(Self {
            music_dir: dir.as_ref().to_path_buf(),
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            current: None,
            current_actual: None,
            stopped: None,
            queue: PlayQueue::new(),
            state: JukeboxState::Stop,
            audio_file_receiver: None,
            audio_decode_handle: None,
            audio_write_handles: Vec::new(),
            events,
            sink,
            _stream: stream,
        })
    }

    pub fn load(&mut self) {
        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(self.music_dir.as_path(), true, sender);
        self.audio_file_receiver = Some(receiver);
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn get(&self, id: TrackId) -> Option<&Track> {
        self.tracks.get(&id)
    }

    pub fn get_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(&id)
    }

    pub fn get_id_from_index(&self, i: usize) -> Option<TrackId> {
        self.tracks.keys().nth(i).copied()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.tracks
            .sort_unstable_by(|_, track1, _, track2| sort.cmp(track1, track2));
        self.sort = sort;
    }

    pub const fn current_track_id(&self) -> Option<TrackId> {
        self.current_actual
    }

    pub fn current_track_pos(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn queue_iter(&self) -> impl ExactSizeIterator<Item = TrackId> {
        self.queue.iter().copied()
    }

    pub fn enqueue_front(&mut self, id: TrackId) {
        self.queue.push_front(id);
    }

    pub fn enqueue_back(&mut self, id: TrackId) {
        self.queue.push_back(id);
    }

    pub fn update(&mut self) {
        self.receive_audio_files();

        let mut render = false;

        // Poll thread handle for audio decoding
        if let Some(handle) = self.audio_decode_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.audio_decode_handle.take().unwrap();
                render = true;
                match handle.join().unwrap() {
                    Ok((id, decoded_audio)) => {
                        self.sink.clear();
                        self.sink.append(decoded_audio);
                        if self.state != JukeboxState::Pause {
                            self.state = JukeboxState::Play;
                            self.sink.play();
                        }
                        self.current_actual = Some(id);
                    }
                    Err(err) => {
                        let log = Log::new(err);
                        self.events.send(AppEvent::Log(log));
                        match self.state {
                            JukeboxState::Play | JukeboxState::Next => {
                                self.play_next();
                            }
                            JukeboxState::Prev => {
                                self.play_previous();
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
                    let log = Log::new(err);
                    self.events.send(AppEvent::Log(log));
                }
            }
        }

        // Play next when idle and finished
        let is_finished = self.sink.empty() && !self.sink.is_paused();
        if self.state == JukeboxState::Play && is_finished {
            self.play_next();
        }

        if render {
            self.events.send(AppEvent::Render);
        }
    }

    pub fn play_track(&mut self, id: TrackId) {
        // TODO: If new track is same as current, simply rewind.
        if let Some(current_id) = self.current_track_id() {
            self.queue.add_to_history(current_id);
        }
        self.state = JukeboxState::Track;
        self.start_play(id);
    }

    pub fn play(&mut self) {
        self.state = JukeboxState::Play;
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.state = JukeboxState::Pause;
        self.sink.pause();
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            match self.stopped.take() {
                Some(id) => {
                    self.play_track(id);
                }
                None => {
                    self.play();
                }
            }
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        // TODO: Should stop also clear queue and history?
        if let Some(id) = self.current_actual.take() {
            self.stopped = Some(id);
            self.events.send(AppEvent::UpdateAndRender);
        }

        self.sink.clear();
        self.current = None;
        self.state = JukeboxState::Stop;
        self.audio_decode_handle = None;
    }

    pub fn play_next(&mut self) {
        if let Some(id) = self.queue.next(self.tracks.len(), self.current) {
            self.state = JukeboxState::Next;
            self.start_play(id);
        }
    }

    pub fn play_previous(&mut self) {
        if let Some(id) = self.queue.previous(self.current) {
            self.state = JukeboxState::Prev;
            self.start_play(id);
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

    fn start_play(&mut self, id: TrackId) {
        self.current = Some(id);
        self.stopped = None;
        self.audio_decode_handle = Some(self.decode_audio(id));
        self.events.send(AppEvent::UpdateAndRender);
    }

    fn decode_audio(&mut self, id: TrackId) -> AudioDecodeHandle {
        let track = self.tracks.get(&id).unwrap();
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
            Ok((id, source))
        })
    }

    fn write_rating(&mut self, id: TrackId, rating: AudioRating) -> AudioWriteHandle {
        let track = self.tracks.get(&id).unwrap();
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

    fn receive_audio_files(&mut self) {
        let Some(receiver) = self.audio_file_receiver.as_ref() else {
            return;
        };

        // Receive processed audio files and convert to tracks
        loop {
            match receiver.try_recv() {
                Ok(audio_file_res) => {
                    let track_res = audio_file_res.and_then(|(audio_file, extension)| {
                        let track = Track::new(
                            audio_file.metadata()?,
                            audio_file.properties(),
                            audio_file.path().to_path_buf(),
                            extension,
                        );
                        Ok(track)
                    });
                    match track_res {
                        Ok(track) => {
                            let last_id = self.tracks.len() as u64;
                            self.tracks.insert_sorted_by(
                                TrackId(last_id),
                                track,
                                |_, track1, _, track2| self.sort.cmp(track1, track2),
                            );
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            self.events.send(AppEvent::Log(log));
                        }
                    }
                }
                Err(err) => match err {
                    mpsc::TryRecvError::Empty => {
                        break;
                    }
                    mpsc::TryRecvError::Disconnected => {
                        self.audio_file_receiver = None;
                        break;
                    }
                },
            }
        }
    }

    pub fn shutdown(self) {
        // Gracefully shutdown by waiting for threads to finish writing tag
        for handle in self.audio_write_handles {
            let _ = handle.join().unwrap();
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(u64);

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    extension: AudioFileExtension,
    duration_display: String,
}

impl Track {
    fn new(
        metadata: AudioMetadata,
        properties: AudioProperties,
        path: PathBuf,
        extension: AudioFileExtension,
    ) -> Self {
        let duration_display = utils::format_duration(properties.duration());

        Self {
            metadata,
            properties,
            path,
            extension,
            duration_display,
        }
    }

    pub const fn title(&self) -> &str {
        self.metadata.title()
    }

    pub const fn artist(&self) -> &str {
        self.metadata.artist()
    }

    pub const fn album(&self) -> &str {
        self.metadata.album()
    }

    pub const fn rating(&self) -> Option<AudioRating> {
        self.metadata.rating()
    }

    pub const fn rating_display(&self) -> &str {
        match self.metadata.rating() {
            Some(rating) => match rating {
                AudioRating::Awful => "★",
                AudioRating::Bad => "★★",
                AudioRating::Ok => "★★★",
                AudioRating::Good => "★★★★",
                AudioRating::Amazing => "★★★★★",
            },
            None => "",
        }
    }

    pub const fn set_rating(&mut self, rating: Option<AudioRating>) {
        self.metadata.set_rating(rating);
    }

    pub const fn duration(&self) -> Duration {
        self.properties.duration()
    }

    pub const fn duration_display(&self) -> &str {
        self.duration_display.as_str()
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub const fn extension(&self) -> AudioFileExtension {
        self.extension
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TrackSort {
    TitleAscending,
    TitleDescending,
    ArtistAscending,
    ArtistDescending,
    #[default]
    AlbumAscending,
    AlbumDescending,
    TimeAscending,
    TimeDescending,
    RatingAscending,
    RatingDescending,
}

impl TrackSort {
    pub const fn next(self) -> Self {
        match self {
            Self::TitleAscending => Self::TitleDescending,
            Self::TitleDescending => Self::ArtistAscending,
            Self::ArtistAscending => Self::ArtistDescending,
            Self::ArtistDescending => Self::AlbumAscending,
            Self::AlbumAscending => Self::AlbumDescending,
            Self::AlbumDescending => Self::TimeAscending,
            Self::TimeAscending => Self::TimeDescending,
            Self::TimeDescending => Self::RatingAscending,
            Self::RatingAscending => Self::RatingDescending,
            Self::RatingDescending => Self::TitleAscending,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::TitleAscending => Self::RatingDescending,
            Self::TitleDescending => Self::TitleAscending,
            Self::ArtistAscending => Self::TitleDescending,
            Self::ArtistDescending => Self::ArtistAscending,
            Self::AlbumAscending => Self::ArtistDescending,
            Self::AlbumDescending => Self::AlbumAscending,
            Self::TimeAscending => Self::AlbumDescending,
            Self::TimeDescending => Self::TimeAscending,
            Self::RatingAscending => Self::TimeDescending,
            Self::RatingDescending => Self::RatingAscending,
        }
    }

    fn cmp(self, t1: &Track, t2: &Track) -> Ordering {
        match self {
            Self::TitleAscending => t1.title().cmp(t2.title()),
            Self::TitleDescending => t2.title().cmp(t1.title()),
            Self::ArtistAscending => t1.artist().cmp(t2.artist()),
            Self::ArtistDescending => t2.artist().cmp(t1.artist()),
            Self::AlbumAscending => t1.album().cmp(t2.album()),
            Self::AlbumDescending => t2.album().cmp(t1.album()),
            Self::TimeAscending => t1.duration().cmp(&t2.duration()),
            Self::TimeDescending => t2.duration().cmp(&t1.duration()),
            Self::RatingAscending => {
                let r1 = t1.rating().map(|r| r as u8).unwrap_or(0);
                let r2 = t2.rating().map(|r| r as u8).unwrap_or(0);
                r1.cmp(&r2)
            }
            Self::RatingDescending => {
                let r1 = t1.rating().map(|r| r as u8).unwrap_or(0);
                let r2 = t2.rating().map(|r| r as u8).unwrap_or(0);
                r2.cmp(&r1)
            }
        }
    }
}

// TODO: Add max queue and history length.
// Just truncate/remove when reaching a certain amount.

struct PlayQueue {
    queue: VecDeque<TrackId>,
    history: Vec<TrackId>,
}

impl PlayQueue {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            history: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn iter(&self) -> std::collections::vec_deque::Iter<'_, TrackId> {
        self.queue.iter()
    }

    fn push_back(&mut self, id: TrackId) {
        self.queue.push_back(id);
    }

    fn push_front(&mut self, id: TrackId) {
        self.queue.push_front(id);
    }

    fn add_to_history(&mut self, id: TrackId) {
        self.history.push(id);
    }

    fn next(&mut self, tracks_len: usize, current_track: Option<TrackId>) -> Option<TrackId> {
        if tracks_len == 0 {
            return None;
        }

        if let Some(id) = current_track {
            self.history.push(id);
        }

        self.queue
            .pop_front()
            .unwrap_or_else(|| TrackId(fastrand::u64(0..tracks_len as u64)))
            .into()
    }

    fn previous(&mut self, current_track: Option<TrackId>) -> Option<TrackId> {
        if let Some(previous_id) = self.history.pop() {
            if let Some(current_id) = current_track {
                self.queue.push_front(current_id);
            }
            return Some(previous_id);
        }

        None
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_queue() {
        const TRACKS_LEN: usize = 3;
        let mut queue = PlayQueue::new();
        let mut current = None;

        for i in 0..TRACKS_LEN {
            queue.push_back(TrackId(i as u64));
        }

        assert_eq!(current, None);
        assert_eq!(queue.queue.len(), TRACKS_LEN);
        assert_eq!(queue.history.len(), 0);

        // Next
        current = queue.next(TRACKS_LEN, current);
        assert_eq!(current, Some(TrackId(0)));
        assert_eq!(queue.queue.len(), 2);
        assert_eq!(queue.history.len(), 0);

        current = queue.next(TRACKS_LEN, current);
        assert_eq!(current, Some(TrackId(1)));
        assert_eq!(queue.queue.len(), 1);
        assert_eq!(queue.history.len(), 1);

        current = queue.next(TRACKS_LEN, current);
        assert_eq!(current, Some(TrackId(2)));
        assert_eq!(queue.queue.len(), 0);
        assert_eq!(queue.history.len(), 2);

        // Previous
        current = queue.previous(current);
        assert_eq!(current, Some(TrackId(1)));
        assert_eq!(queue.queue.len(), 1);
        assert_eq!(queue.history.len(), 1);

        current = queue.previous(current);
        assert_eq!(current, Some(TrackId(0)));
        assert_eq!(queue.queue.len(), 2);
        assert_eq!(queue.history.len(), 0);

        current = queue.previous(current);
        assert_eq!(current, None);
        assert_eq!(queue.queue.len(), 2);
        assert_eq!(queue.history.len(), 0);

        // Next
        current = queue.next(TRACKS_LEN, current);
        assert_eq!(current, Some(TrackId(1)));
        assert_eq!(queue.queue.len(), 1);
        assert_eq!(queue.history.len(), 0);

        current = queue.next(TRACKS_LEN, current);
        assert_eq!(current, Some(TrackId(2)));
        assert_eq!(queue.queue.len(), 0);
        assert_eq!(queue.history.len(), 1);

        // Previous
        current = queue.previous(current);
        assert_eq!(current, Some(TrackId(1)));
        assert_eq!(queue.queue.len(), 1);
        assert_eq!(queue.history.len(), 0);
    }
}
