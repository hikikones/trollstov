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

pub(super) struct Database {
    music_dir: PathBuf,
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    audio_file_receiver: Option<AudioFileReceiver>,
}

impl Database {
    pub(super) fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            music_dir: dir.as_ref().to_path_buf(),
            tracks: IndexMap::new(),
            sort: TrackSort::default(),
            audio_file_receiver: None,
        }
    }

    pub(super) fn load(&mut self) {
        let (sender, receiver) = mpsc::channel();
        traverse_and_process_audio_files(self.music_dir.as_path(), true, sender);
        self.audio_file_receiver = Some(receiver);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.tracks.len()
    }

    pub(super) fn get(&self, id: TrackId) -> Option<&Track> {
        self.tracks.get(&id)
    }

    pub(super) fn get_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(&id)
    }

    pub(super) fn get_id_from_index(&self, i: usize) -> Option<TrackId> {
        self.tracks.keys().nth(i).copied()
    }

    pub(super) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    pub(super) const fn get_sort(&self) -> TrackSort {
        self.sort
    }

    pub(super) fn sort(&mut self, sort: TrackSort) {
        self.tracks
            .sort_unstable_by(|_, track1, _, track2| sort.cmp(track1, track2));
        self.sort = sort;
    }

    pub(super) fn update(&mut self, mut on_error: impl FnMut(AudioFileReport)) {
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
                            on_error(err);
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
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub(super) u64);

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
        let duration_display = format_duration(properties.duration());

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

fn traverse_and_process_audio_files(
    root: impl AsRef<Path>,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    let root = root.as_ref().to_path_buf();
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
    root: impl AsRef<Path>,
    follow_links: bool,
    sender: mpsc::Sender<Result<(AudioFile, AudioFileExtension), AudioFileReport>>,
) {
    let root = root.as_ref().to_path_buf();
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

fn format_duration(duration: Duration) -> String {
    let mut s = String::with_capacity(5);

    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();

    if minutes < 10 {
        s.push('0');
        s.push_str(buffer.format(minutes));
    } else if minutes < 100 {
        s.push_str(buffer.format(minutes));
    } else {
        s.push_str("99:99");
        return s;
    }

    s.push(':');

    if seconds < 10 {
        s.push('0');
        s.push_str(buffer.format(seconds));
    } else {
        s.push_str(buffer.format(seconds));
    }

    s
}
