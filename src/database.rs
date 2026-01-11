use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use indexmap::IndexMap;
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    flac::FlacFile,
    mpeg::MpegFile,
    ogg::OpusFile,
};

use crate::audio::*;

#[derive(Debug)]
pub struct Database {
    tracks: IndexMap<TrackId, Track>,
    sort: TrackSort,
}

impl Database {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let mut tracks = IndexMap::new();

        traverse_audio_files(dir)
            .take(60) // todo: process in another thread
            .map(|(path, audio_format)| {
                let mut file = File::open(&path).unwrap();

                let (metadata, properties) = match audio_format {
                    AudioFileFormat::Mp3 => {
                        let mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            AudioMetadata::from_id3v2(mpeg.id3v2().unwrap()),
                            AudioProperties::from_mpeg(&mpeg),
                        )
                    }
                    AudioFileFormat::Flac => {
                        let flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            AudioMetadata::from_vorbis_comments(flac.vorbis_comments().unwrap()),
                            AudioProperties::from_flac(&flac),
                        )
                    }
                    AudioFileFormat::Opus => {
                        let opus = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            AudioMetadata::from_vorbis_comments(opus.vorbis_comments()),
                            AudioProperties::from_opus(&opus),
                        )
                    }
                };

                Track::new(metadata, properties, path, audio_format)
            })
            .enumerate()
            .for_each(|(i, track)| {
                tracks.insert(TrackId(i as u64), track);
            });

        let sort = TrackSort::Album;
        let mut database = Self { tracks, sort };
        database.sort(sort);
        database
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

    pub fn update(&mut self, id: TrackId, func: impl FnOnce(&mut Track)) {
        if let Some(track) = self.tracks.get_mut(&id) {
            func(track);
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackId, &Track)> + DoubleEndedIterator {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    pub fn values(&self) -> indexmap::map::Values<'_, TrackId, Track> {
        self.tracks.values()
    }

    pub fn values_mut(&mut self) -> indexmap::map::ValuesMut<'_, TrackId, Track> {
        self.tracks.values_mut()
    }

    pub fn sort(&mut self, sort: TrackSort) {
        self.tracks
            .sort_unstable_by(|_, track1, _, track2| match sort {
                TrackSort::Title => track1.title().cmp(track2.title()),
                TrackSort::Artist => track1.artist().cmp(track2.artist()),
                TrackSort::Album => track1.album().cmp(track2.album()),
            });
    }

    fn last_id(&self) -> TrackId {
        self.tracks.keys().last().copied().unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(u64);

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    audio_format: AudioFileFormat,
    duration_display: String,
}

impl Track {
    fn new(
        metadata: AudioMetadata,
        properties: AudioProperties,
        path: PathBuf,
        audio_format: AudioFileFormat,
    ) -> Self {
        let duration = properties.duration();
        let seconds = duration.as_secs() % 60;
        let duration_display = format!("{:02}:{:02}", (duration.as_secs() - seconds) / 60, seconds);

        Self {
            metadata,
            properties,
            path,
            audio_format,
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
        let mut file = File::open(&self.path)?;

        let new_rating = match self.audio_format {
            AudioFileFormat::Mp3 => {
                let mut mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let id3v2 = mpeg.id3v2_mut().unwrap();
                let new_rating =
                    AudioMetadata::set_rating_id3v2(id3v2, self.metadata.rating(), rating);
                mpeg.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
            AudioFileFormat::Flac => {
                let mut flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let vorbis_comments = flac.vorbis_comments_mut().unwrap();
                let new_rating = AudioMetadata::set_rating_vorbis_comments(
                    vorbis_comments,
                    self.metadata.rating(),
                    rating,
                );
                flac.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
            AudioFileFormat::Opus => {
                let mut opus = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let vorbis_comments = opus.vorbis_comments_mut();
                let new_rating = AudioMetadata::set_rating_vorbis_comments(
                    vorbis_comments,
                    self.metadata.rating(),
                    rating,
                );
                opus.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
        };

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

#[derive(Debug, Clone, Copy)]
pub enum TrackSort {
    Title,
    Artist,
    Album,
}

fn traverse_audio_files(
    root: impl AsRef<Path>,
) -> impl Iterator<Item = (PathBuf, AudioFileFormat)> {
    walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|file| file.into_path())
        .filter_map(move |path| match path.extension() {
            Some(file_ext) => {
                if file_ext.eq_ignore_ascii_case("flac") {
                    Some((path, AudioFileFormat::Flac))
                } else if file_ext.eq_ignore_ascii_case("opus") {
                    Some((path, AudioFileFormat::Opus))
                } else if file_ext.eq_ignore_ascii_case("mp3") {
                    Some((path, AudioFileFormat::Mp3))
                } else {
                    None
                }
            }
            None => None,
        })
}
