use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    time::Duration,
};

use super::{AudioFileExtension, AudioMetadata, AudioProperties, AudioRating};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u64);

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    extension: AudioFileExtension,
    duration_utf8_bytes: [u8; 20],
}

impl Track {
    pub(crate) fn new(
        metadata: AudioMetadata,
        properties: AudioProperties,
        path: PathBuf,
        extension: AudioFileExtension,
    ) -> Self {
        // TODO: Make a TrackDuration struct for this.
        let chars = utils::format_duration_on_stack(properties.duration());
        let mut bytes = [0; 20];
        let mut p = 0;
        for c in chars {
            p += c.encode_utf8(&mut bytes[p..]).len();
        }

        Self {
            metadata,
            properties,
            path,
            extension,
            duration_utf8_bytes: bytes,
        }
    }

    pub fn title(&self) -> &str {
        self.metadata.title()
    }

    pub fn artist(&self) -> &str {
        self.metadata.artist()
    }

    pub fn album(&self) -> &str {
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
        unsafe { std::str::from_utf8_unchecked(&self.duration_utf8_bytes) }
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

    pub fn field_str(&self, sort: TrackSort) -> &str {
        match sort {
            TrackSort::Title => self.metadata.title(),
            TrackSort::Artist => self.metadata.artist(),
            TrackSort::Album => self.metadata.album(),
            TrackSort::Time => self.duration_display(),
            TrackSort::Rating => self.metadata.rating().stars(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TrackSort {
    Title,
    Artist,
    #[default]
    Album,
    Time,
    Rating,
}

impl TrackSort {
    pub const fn next(self) -> Self {
        match self {
            Self::Title => Self::Artist,
            Self::Artist => Self::Album,
            Self::Album => Self::Time,
            Self::Time => Self::Rating,
            Self::Rating => Self::Title,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Title => Self::Rating,
            Self::Artist => Self::Title,
            Self::Album => Self::Artist,
            Self::Time => Self::Album,
            Self::Rating => Self::Time,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Title => "Rating",
            Self::Artist => "Title",
            Self::Album => "Artist",
            Self::Time => "Album",
            Self::Rating => "Time",
        }
    }

    // TODO: What about albums with same name?
    // Also, title should be sorted by cd/track listing for each album.
    pub(crate) fn cmp(self, t1: &Track, t2: &Track) -> Ordering {
        match self {
            Self::Title => t1.title().cmp(t2.title()),
            Self::Artist => t1.artist().cmp(t2.artist()),
            Self::Album => t1.album().cmp(t2.album()),
            Self::Time => t1.duration().cmp(&t2.duration()),
            Self::Rating => t1.rating().cmp(&t2.rating()),
        }
    }
}
