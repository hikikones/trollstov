use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use indexmap::IndexMap;

use crate::{
    AudioFile, AudioFileExtension, AudioFileReport, AudioMetadata, AudioProperties, AudioRating,
};

type AudioFileReceiver = mpsc::Receiver<Result<(AudioFile, AudioFileExtension), AudioFileReport>>;

pub struct Database {
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    matcher: Matcher,
    buffer: String,
    receiver: Option<AudioFileReceiver>,
}

impl Database {
    pub fn new(music_dir: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(music_dir, true, sender);

        Self {
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            matcher: Matcher::new(),
            buffer: String::new(),
            receiver: Some(receiver),
        }
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

    pub fn get_index_from_id(&self, id: TrackId) -> Option<usize> {
        self.tracks.get_index_of(&id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> {
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

    pub fn search(&mut self, needle: &str) -> impl Iterator<Item = (TrackId, u16)> {
        self.matcher.update(needle);
        self.tracks.iter().filter_map(|(id, track)| {
            self.buffer
                .extend([track.artist(), " ", track.album(), " ", track.title()]);
            let score = self.matcher.score(&self.buffer);
            self.buffer.clear();
            score.map(|score| (*id, score))
        })
    }

    pub fn update(&mut self, mut on_error: impl FnMut(AudioFileReport)) {
        let Some(receiver) = self.receiver.as_ref() else {
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
                            on_error(err);
                        }
                    }
                }
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
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub(crate) u64);

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
        let duration_display = crate::utils::format_duration(properties.duration);

        Self {
            metadata,
            properties,
            path,
            extension,
            duration_display,
        }
    }

    pub const fn title(&self) -> &str {
        self.metadata.title.as_str()
    }

    pub const fn artist(&self) -> &str {
        self.metadata.artist.as_str()
    }

    pub const fn album(&self) -> &str {
        self.metadata.album.as_str()
    }

    pub const fn rating(&self) -> AudioRating {
        self.metadata.rating
    }

    pub const fn set_rating(&mut self, rating: AudioRating) {
        self.metadata.rating = rating;
    }

    pub const fn duration(&self) -> Duration {
        self.properties.duration
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

    /// Audio bit rate in kbps.
    pub const fn bit_rate(&self) -> u32 {
        self.properties.bit_rate_kbps
    }

    /// Bits per sample, usually 16 or 24 bit.
    pub const fn bit_depth(&self) -> Option<u8> {
        self.properties.bit_depth
    }

    /// Sample rate in kHz.
    pub const fn sample_rate(&self) -> Option<u32> {
        self.properties.sample_rate_khz
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
            Self::RatingAscending => t1.rating().cmp(&t2.rating()),
            Self::RatingDescending => t2.rating().cmp(&t1.rating()),
        }
    }
}

struct Matcher {
    matcher: nucleo_matcher::Matcher,
    atom: nucleo_matcher::pattern::Atom,
    buffer: Vec<char>,
}

impl Matcher {
    fn new() -> Self {
        Self {
            matcher: nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            atom: Self::create_atom(""),
            buffer: Vec::new(),
        }
    }

    fn update(&mut self, needle: &str) {
        self.atom = Self::create_atom(needle);
    }

    fn score(&mut self, haystack: &str) -> Option<u16> {
        self.atom.score(
            nucleo_matcher::Utf32Str::new(haystack, &mut self.buffer),
            &mut self.matcher,
        )
    }

    fn create_atom(needle: &str) -> nucleo_matcher::pattern::Atom {
        nucleo_matcher::pattern::Atom::new(
            needle,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Smart,
            nucleo_matcher::pattern::AtomKind::Fuzzy,
            true,
        )
    }
}

fn traverse_and_process_audio_files(
    root: PathBuf,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    std::thread::spawn(move || {
        walkdir::WalkDir::new(root)
            .follow_links(follow_links)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|file| {
                AudioFileExtension::from_path(file.path()).map(|ext| (file.into_path(), ext))
            })
            .for_each(|(path, extension)| {
                let audio_file =
                    AudioFile::read_from(path, extension).map(|audio_file| (audio_file, extension));
                let _ = sender.send(audio_file);
            });
    });
}

fn _traverse_and_process_audio_files_in_parallel(
    root: PathBuf,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    std::thread::spawn(move || {
        ignore::WalkBuilder::new(root)
            .follow_links(follow_links)
            .build_parallel()
            .run(|| {
                let sender = sender.clone();
                Box::new(move |result| {
                    if let Ok(dir_entry) = result {
                        if let Some(file_type) = dir_entry.file_type() {
                            if file_type.is_file() {
                                if let Some(extension) =
                                    AudioFileExtension::from_path(dir_entry.path())
                                {
                                    let audio_file =
                                        AudioFile::read_from(dir_entry.into_path(), extension)
                                            .map(|audio_file| (audio_file, extension));
                                    let _ = sender.send(audio_file);
                                }
                            }
                        }
                    }

                    ignore::WalkState::Continue
                })
            });
    });
}
