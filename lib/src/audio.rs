use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::{AudioFile as LoftyAudioFile, TaggedFile, TaggedFileExt},
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, PopularimeterFrame},
    mpeg::MpegFile,
    ogg::{OpusFile, VorbisComments, VorbisFile},
    picture::PictureType,
    tag::Accessor,
};

pub struct AudioFile {
    format: AudioFileFormat,
    path: PathBuf,
}

enum AudioFileFormat {
    Flac(FlacFile),
    Opus(OpusFile),
    Vorbis(VorbisFile),
    Mpeg(MpegFile),
}

impl AudioFile {
    pub fn read_from(
        path: impl AsRef<Path>,
        extension: AudioFileExtension,
    ) -> Result<Self, AudioFileReport> {
        let path = path.as_ref();
        let mut file = File::open(path).map_err(|err| {
            AudioFileReport(format!(
                "Failed to open \"{}\" due to {}",
                path.display(),
                err
            ))
        })?;
        let audio_format = match extension {
            AudioFileExtension::Flac => {
                let flac = FlacFile::read_from(&mut file, ParseOptions::new()).map_err(|err| {
                    AudioFileReport(format!(
                        "Failed to read \"{}\" due to {}",
                        path.display(),
                        err
                    ))
                })?;
                AudioFileFormat::Flac(flac)
            }
            AudioFileExtension::Opus => {
                let opus = OpusFile::read_from(&mut file, ParseOptions::new()).map_err(|err| {
                    AudioFileReport(format!(
                        "Failed to read \"{}\" due to {}",
                        path.display(),
                        err
                    ))
                })?;
                AudioFileFormat::Opus(opus)
            }
            AudioFileExtension::Ogg => {
                let ogg_vorbis =
                    VorbisFile::read_from(&mut file, ParseOptions::new()).map_err(|err| {
                        AudioFileReport(format!(
                            "Failed to read \"{}\" due to {}",
                            path.display(),
                            err
                        ))
                    })?;
                AudioFileFormat::Vorbis(ogg_vorbis)
            }
            AudioFileExtension::Mp3 => {
                let mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).map_err(|err| {
                    AudioFileReport(format!(
                        "Failed to read \"{}\" due to {}",
                        path.display(),
                        err
                    ))
                })?;
                AudioFileFormat::Mpeg(mpeg)
            }
        };

        Ok(Self {
            format: audio_format,
            path: path.to_path_buf(),
        })
    }

    pub fn metadata(&self) -> Result<AudioMetadata, AudioFileReport> {
        match &self.format {
            AudioFileFormat::Flac(flac) => match flac.vorbis_comments() {
                Some(vorbis_comments) => Ok(AudioMetadata::from_vorbis_comments(vorbis_comments)),
                None => Err(AudioFileReport(format!(
                    "Unable to extract metadata from \"{}\" due to missing Vorbis tag",
                    self.path.display()
                ))),
            },
            AudioFileFormat::Opus(opus) => {
                let vorbis_comments = opus.vorbis_comments();
                Ok(AudioMetadata::from_vorbis_comments(vorbis_comments))
            }
            AudioFileFormat::Vorbis(vorbis) => {
                let vorbis_comments = vorbis.vorbis_comments();
                Ok(AudioMetadata::from_vorbis_comments(vorbis_comments))
            }
            AudioFileFormat::Mpeg(mpeg) => match mpeg.id3v2() {
                Some(id3v2) => Ok(AudioMetadata::from_id3v2(id3v2)),
                None => Err(AudioFileReport(format!(
                    "Unable to extract metadata from \"{}\" due to missing ID3v2 tag",
                    self.path.display()
                ))),
            },
        }
    }

    pub fn properties(&self) -> AudioProperties {
        match &self.format {
            AudioFileFormat::Flac(flac) => AudioProperties::from_flac(flac),
            AudioFileFormat::Opus(opus) => AudioProperties::from_opus(opus),
            AudioFileFormat::Vorbis(vorbis) => AudioProperties::from_vorbis(vorbis),
            AudioFileFormat::Mpeg(mpeg) => AudioProperties::from_mpeg(mpeg),
        }
    }

    pub fn write_rating(&mut self, rating: Option<AudioRating>) -> Result<(), AudioFileReport> {
        match &mut self.format {
            AudioFileFormat::Flac(flac) => match flac.vorbis_comments_mut() {
                Some(vorbis_comments) => {
                    AudioMetadata::set_vorbis_rating(vorbis_comments, rating);
                    Ok(flac
                        .save_to_path(&self.path, WriteOptions::new())
                        .map_err(|err| {
                            AudioFileReport(format!(
                                "Failed to save \"{}\" due to {}",
                                self.path.display(),
                                err
                            ))
                        })?)
                }
                None => Err(AudioFileReport(format!(
                    "Unable to write rating for \"{}\" due to missing Vorbis tag",
                    self.path.display()
                ))),
            },
            AudioFileFormat::Opus(opus) => {
                AudioMetadata::set_vorbis_rating(opus.vorbis_comments_mut(), rating);
                Ok(opus
                    .save_to_path(&self.path, WriteOptions::new())
                    .map_err(|err| {
                        AudioFileReport(format!(
                            "Failed to save \"{}\" due to {}",
                            self.path.display(),
                            err
                        ))
                    })?)
            }
            AudioFileFormat::Vorbis(vorbis) => {
                AudioMetadata::set_vorbis_rating(vorbis.vorbis_comments_mut(), rating);
                Ok(vorbis
                    .save_to_path(&self.path, WriteOptions::new())
                    .map_err(|err| {
                        AudioFileReport(format!(
                            "Failed to save \"{}\" due to {}",
                            self.path.display(),
                            err
                        ))
                    })?)
            }
            AudioFileFormat::Mpeg(mpeg) => match mpeg.id3v2_mut() {
                Some(id3v2) => {
                    AudioMetadata::set_id3v2_rating(id3v2, rating);
                    Ok(mpeg
                        .save_to_path(&self.path, WriteOptions::new())
                        .map_err(|err| {
                            AudioFileReport(format!(
                                "Failed to save \"{}\" due to {}",
                                self.path.display(),
                                err
                            ))
                        })?)
                }
                None => Err(AudioFileReport(format!(
                    "Unable to write rating for \"{}\" due to missing ID3v2 tag",
                    self.path.display()
                ))),
            },
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug)]
pub struct AudioMetadata {
    title: String,
    artist: String,
    album: String,
    rating: Option<AudioRating>,
}

impl AudioMetadata {
    pub const fn title(&self) -> &str {
        self.title.as_str()
    }

    pub const fn artist(&self) -> &str {
        self.artist.as_str()
    }

    pub const fn album(&self) -> &str {
        self.album.as_str()
    }

    pub const fn rating(&self) -> Option<AudioRating> {
        self.rating
    }

    pub const fn set_rating(&mut self, rating: Option<AudioRating>) {
        self.rating = rating;
    }

    fn from_vorbis_comments(vorbis_comments: &VorbisComments) -> Self {
        Self {
            title: vorbis_comments
                .get("TITLE")
                .map(str::to_owned)
                .unwrap_or_default(),
            artist: vorbis_comments
                .get("ARTIST")
                .map(str::to_owned)
                .unwrap_or_default(),
            album: vorbis_comments
                .get("ALBUM")
                .map(str::to_owned)
                .unwrap_or_default(),
            rating: vorbis_comments
                .get("RATING")
                .and_then(|s| AudioRating::from_vorbis_comments(s)),
        }
    }

    fn from_id3v2(id3v2: &Id3v2Tag) -> Self {
        Self {
            title: id3v2
                .title()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            artist: id3v2
                .artist()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            album: id3v2
                .album()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            rating: id3v2.get(&FrameId::Valid(Cow::Borrowed("POPM"))).and_then(
                |frame| match frame {
                    Frame::Popularimeter(popularimeter_frame) => {
                        AudioRating::from_id3v2(popularimeter_frame.rating)
                    }
                    _ => None,
                },
            ),
        }
    }

    fn set_vorbis_rating(vorbis_comments: &mut VorbisComments, rating: Option<AudioRating>) {
        match rating {
            Some(rating) => {
                vorbis_comments.insert(
                    "RATING".to_string(),
                    rating.into_vorbis_comments().to_string(),
                );
            }
            None => {
                let _ = vorbis_comments.remove("RATING");
            }
        }
    }

    fn set_id3v2_rating(id3v2: &mut Id3v2Tag, rating: Option<AudioRating>) {
        match rating {
            Some(rating) => {
                id3v2.insert(Frame::Popularimeter(PopularimeterFrame::new(
                    String::new(),
                    rating.into_id3v2(),
                    0,
                )));
            }
            None => {
                let _ = id3v2.remove(&FrameId::Valid(Cow::Borrowed("POPM")));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AudioRating {
    Awful = 1,
    Bad = 2,
    Ok = 3,
    Good = 4,
    Amazing = 5,
}

impl AudioRating {
    fn from_vorbis_comments(value: &str) -> Option<Self> {
        value.parse::<u8>().ok().map(|num| match num {
            0..=20 => Self::Awful,
            21..=40 => Self::Bad,
            41..=60 => Self::Ok,
            61..=80 => Self::Good,
            81..=255 => Self::Amazing,
        })
    }

    const fn into_vorbis_comments(&self) -> &str {
        match self {
            AudioRating::Awful => "20",
            AudioRating::Bad => "40",
            AudioRating::Ok => "60",
            AudioRating::Good => "80",
            AudioRating::Amazing => "100",
        }
    }

    const fn from_id3v2(rating: u8) -> Option<Self> {
        match rating {
            0 => None, // Unknown
            1..=51 => Some(AudioRating::Awful),
            52..=102 => Some(AudioRating::Bad),
            103..=153 => Some(AudioRating::Ok),
            154..=204 => Some(AudioRating::Good),
            205..=255 => Some(AudioRating::Amazing),
        }
    }

    const fn into_id3v2(&self) -> u8 {
        match self {
            AudioRating::Awful => 50,
            AudioRating::Bad => 100,
            AudioRating::Ok => 150,
            AudioRating::Good => 200,
            AudioRating::Amazing => 250,
        }
    }

    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Awful),
            '2' => Some(Self::Bad),
            '3' => Some(Self::Ok),
            '4' => Some(Self::Good),
            '5' => Some(Self::Amazing),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct AudioProperties {
    duration: Duration,
}

impl AudioProperties {
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    fn from_flac(flac_file: &FlacFile) -> Self {
        let properties = flac_file.properties();
        Self {
            duration: properties.duration(),
        }
    }

    fn from_opus(opus_file: &OpusFile) -> Self {
        let properties = opus_file.properties();
        Self {
            duration: properties.duration(),
        }
    }

    fn from_vorbis(vorbis_file: &VorbisFile) -> Self {
        let properties = vorbis_file.properties();
        Self {
            duration: properties.duration(),
        }
    }

    fn from_mpeg(mpeg_file: &MpegFile) -> Self {
        let properties = mpeg_file.properties();
        Self {
            duration: properties.duration(),
        }
    }
}

pub struct AudioPicture(TaggedFile);

impl AudioPicture {
    pub fn read(audio_file: impl AsRef<Path>) -> Result<Self, AudioFileReport> {
        let path = audio_file.as_ref();
        let tagged_file = lofty::read_from_path(path).map_err(|err| {
            AudioFileReport(format!(
                "Failed to read \"{}\" due to {}",
                path.display(),
                err
            ))
        })?;
        Ok(Self(tagged_file))
    }

    pub fn bytes(&self) -> Option<&[u8]> {
        self.0
            .primary_tag()
            .or_else(|| self.0.first_tag())
            .and_then(|tag| tag.get_picture_type(PictureType::CoverFront))
            .map(|pic| pic.data())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AudioFileExtension {
    Flac,
    Opus,
    Ogg,
    Mp3,
}

impl AudioFileExtension {
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref().extension().and_then(|ext| {
            if ext.eq_ignore_ascii_case("flac") {
                Some(Self::Flac)
            } else if ext.eq_ignore_ascii_case("opus") {
                Some(Self::Opus)
            } else if ext.eq_ignore_ascii_case("ogg") {
                Some(Self::Ogg)
            } else if ext.eq_ignore_ascii_case("mp3") {
                Some(Self::Mp3)
            } else {
                None
            }
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            AudioFileExtension::Flac => "flac",
            AudioFileExtension::Opus => "opus",
            AudioFileExtension::Ogg => "ogg",
            AudioFileExtension::Mp3 => "mp3",
        }
    }
}

#[derive(Debug)]
pub struct AudioFileReport(String);

impl AudioFileReport {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for AudioFileReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl std::error::Error for AudioFileReport {}

impl From<String> for AudioFileReport {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<AudioFileReport> for String {
    fn from(value: AudioFileReport) -> Self {
        value.0
    }
}
