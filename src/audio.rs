use std::{
    borrow::Cow,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::{AudioFile as LoftyAudioFile, TaggedFile, TaggedFileExt},
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, PopularimeterFrame},
    mpeg::MpegFile,
    ogg::{OpusFile, VorbisComments},
    picture::PictureType,
    tag::Accessor,
};

pub struct AudioFile {
    format: AudioFileFormat,
    path: PathBuf,
}

enum AudioFileFormat {
    Flac(FlacFile),
    Mpeg(MpegFile),
    Opus(OpusFile),
}

impl AudioFile {
    pub fn read_from_path_and_extension(
        path: impl AsRef<Path>,
        extension: AudioFileExtension,
    ) -> lofty::error::Result<Self> {
        let mut buffer = BufReader::new(File::open(&path).unwrap());
        let audio_format = match extension {
            AudioFileExtension::Mp3 => {
                let mpeg = MpegFile::read_from(&mut buffer, ParseOptions::new())?;
                AudioFileFormat::Mpeg(mpeg)
            }
            AudioFileExtension::Flac => {
                let flac = FlacFile::read_from(&mut buffer, ParseOptions::new())?;
                AudioFileFormat::Flac(flac)
            }
            AudioFileExtension::Opus => {
                let opus = OpusFile::read_from(&mut buffer, ParseOptions::new())?;
                AudioFileFormat::Opus(opus)
            }
        };

        Ok(Self {
            format: audio_format,
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn metadata(&self) -> AudioMetadata {
        match &self.format {
            AudioFileFormat::Flac(flac) => {
                AudioMetadata::from_vorbis_comments(flac.vorbis_comments().unwrap())
            }
            AudioFileFormat::Mpeg(mpeg) => AudioMetadata::from_id3v2(mpeg.id3v2().unwrap()),
            AudioFileFormat::Opus(opus) => {
                AudioMetadata::from_vorbis_comments(opus.vorbis_comments())
            }
        }
    }

    pub fn properties(&self) -> AudioProperties {
        match &self.format {
            AudioFileFormat::Flac(flac) => AudioProperties::from_flac(flac),
            AudioFileFormat::Mpeg(mpeg) => AudioProperties::from_mpeg(mpeg),
            AudioFileFormat::Opus(opus) => AudioProperties::from_opus(opus),
        }
    }

    pub fn write_rating(&mut self, rating: Option<AudioRating>) -> color_eyre::Result<()> {
        match &mut self.format {
            AudioFileFormat::Mpeg(mpeg) => {
                let id3v2 = mpeg.id3v2_mut().unwrap();
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
                mpeg.save_to_path(&self.path, WriteOptions::new())?;
            }
            AudioFileFormat::Flac(flac) => {
                let vorbis_comments = flac.vorbis_comments_mut().unwrap();
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
                flac.save_to_path(&self.path, WriteOptions::new())?;
            }
            AudioFileFormat::Opus(opus) => {
                let vorbis_comments = opus.vorbis_comments_mut();
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
                opus.save_to_path(&self.path, WriteOptions::new())?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct AudioMetadata {
    title: String,
    artist: String,
    album: String,
    rating: Option<AudioRating>,
    // todo: tag type?
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

    fn from_id3v2(metadata: &Id3v2Tag) -> Self {
        Self {
            title: metadata
                .title()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            artist: metadata
                .artist()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            album: metadata
                .album()
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_default(),
            rating: metadata
                .get(&FrameId::Valid(Cow::Borrowed("POPM")))
                .and_then(|frame| match frame {
                    Frame::Popularimeter(popularimeter_frame) => {
                        Some(AudioRating::from_id3v2(popularimeter_frame.rating))
                    }
                    _ => None,
                }),
        }
    }

    fn from_vorbis_comments(metadata: &VorbisComments) -> Self {
        Self {
            title: metadata.get("TITLE").map(str::to_owned).unwrap_or_default(),
            artist: metadata
                .get("ARTIST")
                .map(str::to_owned)
                .unwrap_or_default(),
            album: metadata.get("ALBUM").map(str::to_owned).unwrap_or_default(),
            rating: metadata
                .get("RATING")
                .and_then(|s| AudioRating::from_vorbis_comments(s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRating {
    Awful,
    Bad,
    Ok,
    Good,
    Amazing,
}

impl AudioRating {
    const fn from_id3v2(rating: u8) -> Self {
        match rating {
            0..=51 => AudioRating::Awful,
            52..=102 => AudioRating::Bad,
            103..=153 => AudioRating::Ok,
            154..=204 => AudioRating::Good,
            205..=255 => AudioRating::Amazing,
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

    fn from_mpeg(mpeg_file: &MpegFile) -> Self {
        let properties = mpeg_file.properties();
        Self {
            duration: properties.duration(),
        }
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
}

#[derive(Debug, Clone, Copy)]
pub enum AudioFileExtension {
    Flac,
    Mp3,
    Opus,
}

pub struct AudioPicture(TaggedFile);

impl AudioPicture {
    pub fn read(audio_file: impl AsRef<Path>) -> color_eyre::Result<Self> {
        let tagged_file = lofty::read_from_path(audio_file)?;
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

pub fn traverse_audio_files(
    root: impl AsRef<Path>,
) -> impl Iterator<Item = (PathBuf, AudioFileExtension)> {
    walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|file| file.into_path())
        .filter_map(move |path| match path.extension() {
            Some(file_ext) => {
                if file_ext.eq_ignore_ascii_case("flac") {
                    Some((path, AudioFileExtension::Flac))
                } else if file_ext.eq_ignore_ascii_case("opus") {
                    Some((path, AudioFileExtension::Opus))
                } else if file_ext.eq_ignore_ascii_case("mp3") {
                    Some((path, AudioFileExtension::Mp3))
                } else {
                    None
                }
            }
            None => None,
        })
}
