use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    time::Duration,
};

use audio::{AudioFileExtension, AudioMetadata, AudioProperties, AudioRating};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u64);

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    extension: AudioFileExtension,
    duration_display: String,
}

impl Track {
    pub(crate) fn new(
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

    pub const fn rating(&self) -> AudioRating {
        self.metadata.rating()
    }

    pub const fn set_rating(&mut self, rating: AudioRating) {
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

    /// Audio bit rate in kbps.
    pub const fn bit_rate(&self) -> u32 {
        self.properties.bit_rate_kbps()
    }

    /// Bits per sample, usually 16 or 24 bit.
    pub const fn bit_depth(&self) -> Option<u8> {
        self.properties.bit_depth()
    }

    /// Sample rate in kHz.
    pub const fn sample_rate(&self) -> Option<u32> {
        self.properties.sample_rate_khz()
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

    pub(crate) fn cmp(self, t1: &Track, t2: &Track) -> Ordering {
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
