use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use indexmap::IndexMap;

use crate::audio::*;

pub struct Jukebox {
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
    current: Option<TrackId>,
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
}

impl Jukebox {
    pub fn new(dir: impl AsRef<Path>) -> color_eyre::Result<Self> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = rodio::Sink::connect_new(stream.mixer());

        // TODO: process in another thread
        let tracks = IndexMap::from_iter(
            traverse_audio_files(dir)
                .take(60)
                .filter_map(|(path, extension)| {
                    match AudioFile::read_from_path_and_extension(&path, extension) {
                        Ok(audio) => {
                            let track =
                                Track::new(audio.metadata(), audio.properties(), path, extension);
                            Some(track)
                        }
                        Err(_) => {
                            //todo
                            None
                        }
                    }
                })
                .enumerate()
                .map(|(i, track)| (TrackId(i as u64), track)),
        );

        let sort = TrackSort::default();
        let mut jukebox = Self {
            tracks,
            sort,
            current: None,
            sink,
            _stream: stream,
        };
        jukebox.sort(sort);
        Ok(jukebox)
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn is_playing(&self) -> bool {
        self.sink.empty()
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

    pub fn update(&mut self, id: TrackId, func: impl FnOnce(&mut Track)) {
        if let Some(track) = self.tracks.get_mut(&id) {
            func(track);
        }
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
            .sort_unstable_by(|_, track1, _, track2| match sort {
                TrackSort::Title => track1.title().cmp(track2.title()),
                TrackSort::Artist => track1.artist().cmp(track2.artist()),
                TrackSort::Album => track1.album().cmp(track2.album()),
                TrackSort::Time => track1.duration_display().cmp(track2.duration_display()),
            });
        self.sort = sort;
    }

    pub const fn current(&self) -> Option<TrackId> {
        self.current
    }

    pub fn play(&mut self, id: TrackId) -> color_eyre::Result<()> {
        let track = self.tracks.get(&id).unwrap();
        let file = BufReader::new(File::open(track.path())?);
        let input = rodio::decoder::Decoder::new(file)?;

        self.sink.clear();
        self.sink.append(input);
        self.sink.play();
        self.current = Some(id);

        Ok(())
    }

    pub fn pause_or_play(&mut self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current = None;
    }

    pub fn play_next(&mut self) {
        todo!()
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
        let duration = properties.duration();
        let seconds = duration.as_secs() % 60;
        let duration_display = format!("{:02}:{:02}", (duration.as_secs() - seconds) / 60, seconds);

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

    pub fn set_rating(&mut self, rating: AudioRating) -> color_eyre::Result<()> {
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
}
