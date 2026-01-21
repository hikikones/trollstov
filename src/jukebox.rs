use std::{
    cmp::Ordering,
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
    events::{AppEvent, EventHandler},
    pages::{Log, LogLevel},
    utils,
};

pub struct Jukebox {
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    current: Option<TrackId>,
    stopped: Option<TrackId>,
    audio_file_receiver:
        Option<mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileError>>>,
    audio_play_handle:
        Option<std::thread::JoinHandle<Result<(TrackId, Decoder<BufReader<File>>), JukeboxError>>>,
    audio_write_handle:
        Option<std::thread::JoinHandle<Result<(TrackId, Option<AudioRating>), AudioFileError>>>,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

impl Jukebox {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(dir, true, sender);

        Ok(Self {
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            current: None,
            stopped: None,
            audio_file_receiver: Some(receiver),
            audio_play_handle: None,
            audio_write_handle: None,
            sink,
            _stream: stream,
        })
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

    pub fn get_key_from_index(&self, i: usize) -> Option<TrackId> {
        self.tracks.keys().nth(i).copied()
    }

    pub fn get_key_value_from_index(&self, i: usize) -> Option<(TrackId, &Track)> {
        self.tracks.iter().nth(i).map(|(id, track)| (*id, track))
    }

    pub fn get_index_from_key(&self, id: TrackId) -> Option<usize> {
        self.keys()
            .enumerate()
            .find(|(_, tid)| *tid == id)
            .map(|(i, _)| i)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    pub fn keys(&self) -> std::iter::Copied<indexmap::map::Keys<'_, TrackId, Track>> {
        self.tracks.keys().copied()
    }

    pub fn values(&self) -> indexmap::map::Values<'_, TrackId, Track> {
        self.tracks.values()
    }

    pub fn values_mut(&mut self) -> indexmap::map::ValuesMut<'_, TrackId, Track> {
        self.tracks.values_mut()
    }

    pub const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.tracks
            .sort_unstable_by(|_, track1, _, track2| sort.cmp(track1, track2));
        self.sort = sort;
    }

    pub const fn current(&self) -> Option<TrackId> {
        self.current
    }

    pub fn pos(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn update(&mut self, events: &EventHandler) {
        // Receive processed audio files and convert to tracks
        if let Some(receiver) = self.audio_file_receiver.as_ref() {
            loop {
                match receiver.try_recv() {
                    Ok(audio_file_res) => {
                        let track_res =
                            audio_file_res.and_then(|(audio_file, extension)| match extension {
                                AudioFileExtension::Flac | AudioFileExtension::Mp3 => {
                                    Ok(Track::new(
                                        audio_file.metadata()?,
                                        audio_file.properties(),
                                        audio_file.path().to_path_buf(),
                                        extension,
                                    ))
                                }
                                AudioFileExtension::Opus => {
                                    Err(AudioFileError::Unsupported(format!(
                                        "Unsupported file {}.",
                                        audio_file.path().to_string_lossy()
                                    )))
                                }
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
                                events.send(AppEvent::Log(log));
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

        // Poll thread handle for audio decoding
        if let Some(handle) = self.audio_play_handle.take() {
            if handle.is_finished() {
                match handle.join().unwrap() {
                    Ok((id, source)) => {
                        self.sink.clear();
                        self.sink.append(source);
                        self.sink.play();
                        self.current = Some(id);
                        self.stopped = None;
                        events.send(AppEvent::UpdateAndRender);
                    }
                    Err(err) => {
                        let log = Log::new(err.to_string(), LogLevel::Error);
                        events.send(AppEvent::Log(log));
                    }
                }
            } else {
                self.audio_play_handle = Some(handle);
            }
        }

        // Poll thread handle for finished tag writing
        if let Some(handle) = self.audio_write_handle.take() {
            if handle.is_finished() {
                match handle.join().unwrap() {
                    Ok((id, new_rating)) => {
                        let track = self.get_mut(id).unwrap();
                        track.set_rating(new_rating);
                        events.send(AppEvent::Render);
                    }
                    Err(err) => {
                        let log = Log::new(err.to_string(), LogLevel::Error);
                        events.send(AppEvent::Log(log));
                    }
                }
            } else {
                self.audio_write_handle = Some(handle);
            }
        }

        // Play next when idle and finished
        if self.sink.empty() && !self.sink.is_paused() {
            self.play_next();
        }
    }

    pub fn play(&mut self, id: TrackId) {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();

        let handle = std::thread::spawn(move || {
            let file = File::open(path)?;
            let buffer = BufReader::new(file);
            let source = Decoder::new(buffer)?;
            Ok((id, source))
        });
        self.audio_play_handle = Some(handle);
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
        self.sink.clear();
        self.stopped = self.current.take();
    }

    pub fn play_next(&mut self) {
        let next_id = fastrand::u64(0..self.tracks.len() as u64);
        self.play(TrackId(next_id));
    }

    pub fn play_previous(&mut self) {
        todo!()
    }

    pub fn set_rating(&mut self, id: TrackId, rating: AudioRating) {
        let track = self.tracks.get(&id).unwrap();
        let path = track.path().to_path_buf();
        let extension = track.extension();
        let current_rating = track.rating();

        let handle = std::thread::spawn(move || {
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
        });
        self.audio_write_handle = Some(handle);
    }

    pub fn shutdown(mut self) {
        // Gracefully shutdown by waiting for thread to finish writing tag
        if let Some(handle) = self.audio_write_handle.take() {
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
        let mut duration_display = String::with_capacity(5);
        utils::format_duration(properties.duration(), &mut duration_display);

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
                AudioRating::Awful => "*",
                AudioRating::Bad => "**",
                AudioRating::Ok => "***",
                AudioRating::Good => "****",
                AudioRating::Amazing => "*****",
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

#[derive(Debug, Default, Clone, Copy)]
pub enum TrackSort {
    Title,
    Artist,
    #[default]
    Album,
    Time,
}

impl TrackSort {
    pub const fn next(self) -> Self {
        match self {
            Self::Title => Self::Artist,
            Self::Artist => Self::Album,
            Self::Album => Self::Time,
            Self::Time => Self::Title,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Title => Self::Time,
            Self::Artist => Self::Title,
            Self::Album => Self::Artist,
            Self::Time => Self::Album,
        }
    }

    fn cmp(self, track1: &Track, track2: &Track) -> Ordering {
        match self {
            TrackSort::Title => track1.title().cmp(track2.title()),
            TrackSort::Artist => track1.artist().cmp(track2.artist()),
            TrackSort::Album => track1.album().cmp(track2.album()),
            TrackSort::Time => track1.duration_display().cmp(track2.duration_display()),
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
