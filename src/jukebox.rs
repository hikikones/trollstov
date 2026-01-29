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
    pages::{Log, LogLevel},
    utils,
};

type AudioFileReceiver = mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileError>>;
type AudioDecodeHandle =
    std::thread::JoinHandle<Result<(TrackId, Decoder<BufReader<File>>), JukeboxError>>;
type AudioWriteHandle =
    std::thread::JoinHandle<Result<(TrackId, Option<AudioRating>), AudioFileError>>;

pub struct Jukebox {
    music_dir: PathBuf,
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    current: Option<TrackId>,
    stopped: Option<TrackId>,
    queue: PlayQueue,
    events: EventSender,
    audio_file_receiver: Option<AudioFileReceiver>,
    audio_play_handle: Option<AudioDecodeHandle>,
    audio_write_handle: Option<AudioWriteHandle>,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
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
            stopped: None,
            queue: PlayQueue::new(),
            events,
            audio_file_receiver: None,
            audio_play_handle: None,
            audio_write_handle: None,
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

    pub const fn current_track(&self) -> Option<TrackId> {
        self.current
    }

    pub fn current_audio_position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn queue_iter(&self) -> impl Iterator<Item = (TrackId, &Track)> {
        self.queue
            .queue
            .iter()
            .filter_map(|id| self.tracks.get(id).map(|track| (*id, track)))
    }

    pub fn enqueue_front(&mut self, id: TrackId) {
        self.queue.push_front(id);
    }

    pub fn enqueue_back(&mut self, id: TrackId) {
        self.queue.push_back(id);
    }

    pub fn update(&mut self) {
        self.receive_audio_files();

        // Poll thread handle for audio decoding
        if let Some(handle) = self.audio_play_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.audio_play_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok((id, source)) => {
                        self.sink.clear();
                        self.sink.append(source);
                        self.sink.play();
                        self.current = Some(id);
                        self.stopped = None;
                        self.events.send(AppEvent::UpdateAndRender);
                    }
                    Err(err) => {
                        let log = Log::new(err.to_string(), LogLevel::Error);
                        self.events.send(AppEvent::Log(log));
                    }
                }
            }
        }

        // Poll thread handle for finished tag writing
        if let Some(handle) = self.audio_write_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.audio_write_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok((id, new_rating)) => {
                        let track = self.get_mut(id).unwrap();
                        track.set_rating(new_rating);
                        self.events.send(AppEvent::Render);
                    }
                    Err(err) => {
                        let log = Log::new(err.to_string(), LogLevel::Error);
                        self.events.send(AppEvent::Log(log));
                    }
                }
            }
        }

        // Play next when idle and finished
        if self.sink.empty() && !self.sink.is_paused() {
            self.play_next();
        }
    }

    pub fn play(&mut self, id: TrackId) {
        // TODO: If new track is same as current, simply rewind.
        self.audio_play_handle = Some(self.decode_audio(id));
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            match self.stopped.take() {
                Some(id) => {
                    self.play(id);
                }
                None => {
                    self.sink.play();
                }
            }
        } else {
            self.sink.pause();
        }
    }

    pub fn stop(&mut self) {
        // TODO: Should stop also clear queue and history?
        if let Some(id) = self.current.take() {
            self.sink.clear();
            self.stopped = Some(id);
            self.events.send(AppEvent::UpdateAndRender);
        }
    }

    pub fn play_next(&mut self) {
        if let Some(id) = self.queue.next(self.tracks.len(), self.current) {
            self.play(id);
        }
    }

    pub fn play_previous(&mut self) {
        if let Some(id) = self.queue.previous(self.current) {
            self.play(id);
        }
    }

    pub fn seek(&mut self, dur: Duration) {
        if self.sink.empty() {
            return;
        }

        let _ = self.sink.try_seek(self.sink.get_pos() + dur);
    }

    pub fn set_rating(&mut self, id: TrackId, rating: AudioRating) {
        // TODO: Write handle should be a list of handles.
        // Just in case you want to rate multiple tracks at once.
        self.audio_write_handle = Some(self.write_rating(id, rating));
    }

    fn decode_audio(&mut self, id: TrackId) -> AudioDecodeHandle {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();

        std::thread::spawn(move || {
            let file = File::open(path)?;
            let buffer = BufReader::new(file);
            let source = Decoder::new(buffer)?;
            Ok((id, source))
        })
    }

    fn write_rating(&mut self, id: TrackId, rating: AudioRating) -> AudioWriteHandle {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();
        let extension = track.extension();
        let current_rating = track.rating();

        std::thread::spawn(move || {
            let mut audio_file = AudioFile::read_from_path_and_extension(path, extension)?;
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
                    let track_res =
                        audio_file_res.and_then(|(audio_file, extension)| match extension {
                            AudioFileExtension::Flac | AudioFileExtension::Mp3 => Ok(Track::new(
                                audio_file.metadata()?,
                                audio_file.properties(),
                                audio_file.path().to_path_buf(),
                                extension,
                            )),
                            AudioFileExtension::Opus => Err(AudioFileError::Unsupported(format!(
                                "Unsupported file \"{}\".",
                                audio_file.path().to_string_lossy()
                            ))),
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
                            let log = match err {
                                AudioFileError::Io(error) => {
                                    Log::new(error.to_string(), LogLevel::Error)
                                }
                                AudioFileError::Lofty(error) => {
                                    Log::new(error.to_string(), LogLevel::Error)
                                }
                                AudioFileError::Metadata(msg) => Log::new(msg, LogLevel::Error),
                                AudioFileError::Unsupported(msg) => {
                                    Log::new(msg, LogLevel::Warning)
                                }
                            };
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

    pub fn shutdown(mut self) {
        // Gracefully shutdown by waiting for thread to finish writing tag
        if let Some(handle) = self.audio_write_handle.take() {
            let _ = handle.join().unwrap();
        }
    }
}

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

    fn push_back(&mut self, id: TrackId) {
        self.queue.push_back(id);
    }

    fn push_front(&mut self, id: TrackId) {
        self.queue.push_front(id);
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

#[derive(Debug)]
pub enum JukeboxError {
    Io(std::io::Error),
    Stream(rodio::StreamError),
    Decode(rodio::decoder::DecoderError),
    Audio(AudioFileError),
}

impl std::fmt::Display for JukeboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JukeboxError::Io(error) => error.fmt(f),
            JukeboxError::Stream(error) => error.fmt(f),
            JukeboxError::Decode(error) => error.fmt(f),
            JukeboxError::Audio(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for JukeboxError {}

impl From<std::io::Error> for JukeboxError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rodio::StreamError> for JukeboxError {
    fn from(error: rodio::StreamError) -> Self {
        Self::Stream(error)
    }
}

impl From<rodio::decoder::DecoderError> for JukeboxError {
    fn from(error: rodio::decoder::DecoderError) -> Self {
        Self::Decode(error)
    }
}

impl From<AudioFileError> for JukeboxError {
    fn from(error: AudioFileError) -> Self {
        Self::Audio(error)
    }
}
