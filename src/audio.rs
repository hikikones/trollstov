use std::{
    borrow::Cow,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, PopularimeterFrame},
    mpeg::MpegFile,
    ogg::{OpusFile, VorbisComments},
    tag::Accessor,
};

pub struct AudioPlayback {
    stream: rodio::OutputStream,
    sink: Option<rodio::Sink>,
}

impl AudioPlayback {
    pub fn new() -> Result<Self, rodio::StreamError> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()?;
        Ok(Self { stream, sink: None })
    }

    pub fn play(&mut self, path: impl AsRef<Path>) -> color_eyre::Result<()> {
        let file = BufReader::new(File::open(path)?);
        self.sink = Some(rodio::play(&self.stream.mixer(), file)?);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Track {
    metadata: AudioMetadata,
    properties: AudioProperties,
    path: PathBuf,
    audio_format: AudioFileFormat,
}

impl Track {
    pub const fn new(
        metadata: AudioMetadata,
        properties: AudioProperties,
        path: PathBuf,
        audio_format: AudioFileFormat,
    ) -> Self {
        Self {
            metadata,
            properties,
            path,
            audio_format,
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

    pub const fn rating(&self) -> Option<AudioRating> {
        self.metadata.rating
    }

    pub const fn rating_display(&self) -> &str {
        match self.metadata.rating {
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
                    AudioMetadata::set_rating_id3v2(id3v2, self.metadata.rating, rating);
                mpeg.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
            AudioFileFormat::Flac => {
                let mut flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let vorbis_comments = flac.vorbis_comments_mut().unwrap();
                let new_rating = AudioMetadata::set_rating_vorbis_comments(
                    vorbis_comments,
                    self.metadata.rating,
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
                    self.metadata.rating,
                    rating,
                );
                opus.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
        };

        self.metadata.rating = new_rating;

        Ok(())
    }

    pub const fn duration(&self) -> Duration {
        self.properties.duration
    }

    pub const fn duration_display(&self) -> &str {
        self.properties.duration_display.as_str()
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
    // todo: tag type?
}

impl AudioMetadata {
    pub fn from_id3v2(metadata: &Id3v2Tag) -> Self {
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

    pub fn from_vorbis_comments(metadata: &VorbisComments) -> Self {
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

    fn set_rating_id3v2(
        id3v2: &mut Id3v2Tag,
        current_rating: Option<AudioRating>,
        new_rating: AudioRating,
    ) -> Option<AudioRating> {
        match current_rating {
            Some(current_rating) => {
                if current_rating != new_rating {
                    // Replace rating when they differ
                    id3v2.insert(Frame::Popularimeter(PopularimeterFrame::new(
                        String::new(),
                        new_rating.into_id3v2(),
                        0,
                    )));
                    Some(new_rating)
                } else {
                    // Remove rating when they are the same
                    let _ = id3v2.remove(&FrameId::Valid(Cow::Borrowed("POPM")));
                    None
                }
            }
            None => {
                // Insert new rating
                id3v2.insert(Frame::Popularimeter(PopularimeterFrame::new(
                    String::new(),
                    new_rating.into_id3v2(),
                    0,
                )));
                Some(new_rating)
            }
        }
    }

    fn set_rating_vorbis_comments(
        vorbis_comments: &mut VorbisComments,
        current_rating: Option<AudioRating>,
        new_rating: AudioRating,
    ) -> Option<AudioRating> {
        match current_rating {
            Some(current_rating) => {
                if current_rating != new_rating {
                    // Replace rating when they differ
                    vorbis_comments.insert(
                        "RATING".to_string(),
                        new_rating.into_vorbis_comments().to_string(),
                    );
                    Some(new_rating)
                } else {
                    // Remove rating when they are the same
                    let _ = vorbis_comments.remove("RATING");
                    None
                }
            }
            None => {
                // Insert new rating
                vorbis_comments.insert(
                    "RATING".to_string(),
                    new_rating.into_vorbis_comments().to_string(),
                );
                Some(new_rating)
            }
        }
    }
}

#[derive(Debug)]
pub struct AudioProperties {
    duration: Duration,
    duration_display: String,
}

impl AudioProperties {
    pub fn from_mpeg(mpeg_file: &MpegFile) -> Self {
        let properties = mpeg_file.properties();
        Self {
            duration: properties.duration(),
            duration_display: duration_display(properties.duration()),
        }
    }

    pub fn from_flac(flac_file: &FlacFile) -> Self {
        let properties = flac_file.properties();
        Self {
            duration: properties.duration(),
            duration_display: duration_display(properties.duration()),
        }
    }

    pub fn from_opus(opus_file: &OpusFile) -> Self {
        let properties = opus_file.properties();
        Self {
            duration: properties.duration(),
            duration_display: duration_display(properties.duration()),
        }
    }
}

fn duration_display(duration: Duration) -> String {
    let seconds = duration.as_secs() % 60;
    format!("{:02}:{:02}", (duration.as_secs() - seconds) / 60, seconds)
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
pub enum AudioFileFormat {
    Flac,
    Mp3,
    Opus,
}
