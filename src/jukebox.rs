use std::{
    cmp::Ordering,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use indexmap::IndexMap;

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
    receiver: Option<mpsc::Receiver<Result<Track, AudioFileError>>>,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

impl Jukebox {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        let (sender, receiver) = mpsc::channel();
        let dir = dir.as_ref().to_path_buf();
        thread::spawn(move || {
            traverse_audio_files(dir)
                .map(|(path, extension)| match extension {
                    AudioFileExtension::Flac | AudioFileExtension::Mp3 => {
                        AudioFile::read_from_path_and_extension(&path, extension).and_then(
                            |audio_file| {
                                Ok(Track::new(
                                    audio_file.metadata()?,
                                    audio_file.properties(),
                                    path,
                                    extension,
                                ))
                            },
                        )
                    }
                    AudioFileExtension::Opus => Err(AudioFileError::Unsupported(format!(
                        "Unsupported file {}.",
                        path.to_string_lossy()
                    ))),
                })
                .for_each(|track| {
                    let _ = sender.send(track);
                });
        });

        Ok(Self {
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            current: None,
            stopped: None,
            receiver: Some(receiver),
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
        self.tracks.keys().copied().nth(i)
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
        if let Some(receiver) = self.receiver.as_ref() {
            loop {
                match receiver.try_recv() {
                    Ok(track_res) => match track_res {
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
                    },
                    Err(err) => match err {
                        mpsc::TryRecvError::Empty => {
                            break;
                        }
                        mpsc::TryRecvError::Disconnected => {
                            self.receiver = None;
                            break;
                        }
                    },
                }
            }
        }

        if self.sink.empty() && !self.sink.is_paused() {
            let _ = self.play_random();
        }
    }

    pub fn play(&mut self, id: TrackId) -> Result<(), Box<dyn std::error::Error>> {
        let track = self.tracks.get(&id).unwrap();
        let file = BufReader::new(File::open(track.path())?);
        let input = rodio::decoder::Decoder::new(file)?;

        self.sink.clear();
        self.sink.append(input);
        self.sink.play();
        self.current = Some(id);
        self.stopped = None;

        Ok(())
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            match self.stopped.take() {
                Some(id) => {
                    let _ = self.play(id);
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

    pub fn play_random(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let next_id = fastrand::u64(0..self.tracks.len() as u64);
        self.play(TrackId(next_id))
    }

    pub fn play_previous(&mut self) {
        todo!()
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

    pub fn set_rating(&mut self, rating: AudioRating) -> Result<(), AudioFileError> {
        let mut audio_file = AudioFile::read_from_path_and_extension(&self.path, self.extension)?;
        let new_rating = match self.metadata.rating() {
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
        self.metadata.set_rating(new_rating);

        Ok(())
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
