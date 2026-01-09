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
pub struct Database {
    tracks: Vec<Track>,
}

impl std::ops::Deref for Database {
    type Target = Vec<Track>;

    fn deref(&self) -> &Self::Target {
        &self.tracks
    }
}

impl std::ops::DerefMut for Database {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tracks
    }
}

impl Database {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let tracks = traverse_audio_files(dir)
            .take(60) // todo: process in another thread
            .map(|(audio_format, path)| {
                let mut file = File::open(&path).unwrap();

                let (metadata, properties) = match audio_format {
                    AudioFileFormat::Mp3 => {
                        let mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            Metadata::from_id3v2(mpeg.id3v2().unwrap()),
                            Properties::from_mpeg(&mpeg),
                        )
                    }
                    AudioFileFormat::Flac => {
                        let flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            Metadata::from_vorbis_comments(flac.vorbis_comments().unwrap()),
                            Properties::from_flac(&flac),
                        )
                    }
                    AudioFileFormat::Opus => {
                        let opus = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        (
                            Metadata::from_vorbis_comments(opus.vorbis_comments()),
                            Properties::from_opus(&opus),
                        )
                    }
                };

                Track {
                    metadata,
                    properties,
                    path,
                    audio_format,
                }
            })
            .collect();

        Self { tracks }
    }
}

#[derive(Debug)]
pub struct Track {
    metadata: Metadata,
    properties: Properties,
    path: PathBuf,
    audio_format: AudioFileFormat,
}

impl Track {
    pub const fn title(&self) -> &str {
        self.metadata.title.as_str()
    }

    pub const fn artist(&self) -> &str {
        self.metadata.artist.as_str()
    }

    pub const fn album(&self) -> &str {
        self.metadata.album.as_str()
    }

    pub const fn rating(&self) -> Option<Rating> {
        self.metadata.rating
    }

    pub const fn rating_display(&self) -> &str {
        match self.metadata.rating {
            Some(rating) => match rating {
                Rating::Awful => "*",
                Rating::Bad => "**",
                Rating::Ok => "***",
                Rating::Good => "****",
                Rating::Amazing => "*****",
            },
            None => "",
        }
    }

    pub fn set_rating(&mut self, rating: Rating) -> color_eyre::Result<()> {
        let mut file = File::open(&self.path)?;

        let new_rating = match self.audio_format {
            AudioFileFormat::Mp3 => {
                let mut mpeg = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let id3v2 = mpeg.id3v2_mut().unwrap();
                let new_rating = Metadata::set_rating_id3v2(id3v2, self.metadata.rating, rating);
                mpeg.save_to_path(&self.path, WriteOptions::new())?;
                new_rating
            }
            AudioFileFormat::Flac => {
                let mut flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                let vorbis_comments = flac.vorbis_comments_mut().unwrap();
                let new_rating = Metadata::set_rating_vorbis_comments(
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
                let new_rating = Metadata::set_rating_vorbis_comments(
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
struct Metadata {
    title: String,
    artist: String,
    album: String,
    rating: Option<Rating>,
    // todo: tag type?
}

impl Metadata {
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
                        Some(Rating::from_id3v2(popularimeter_frame.rating))
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
                .and_then(|s| Rating::from_vorbis_comments(s)),
        }
    }

    fn set_rating_id3v2(
        id3v2: &mut Id3v2Tag,
        current_rating: Option<Rating>,
        new_rating: Rating,
    ) -> Option<Rating> {
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
        current_rating: Option<Rating>,
        new_rating: Rating,
    ) -> Option<Rating> {
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
struct Properties {
    duration: Duration,
    duration_display: String,
}

impl Properties {
    fn from_mpeg(mpeg_file: &MpegFile) -> Self {
        let properties = mpeg_file.properties();
        Self {
            duration: properties.duration(),
            duration_display: duration_display(properties.duration()),
        }
    }

    fn from_flac(flac_file: &FlacFile) -> Self {
        let properties = flac_file.properties();
        Self {
            duration: properties.duration(),
            duration_display: duration_display(properties.duration()),
        }
    }

    fn from_opus(opus_file: &OpusFile) -> Self {
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
pub enum Rating {
    Awful,
    Bad,
    Ok,
    Good,
    Amazing,
}

impl Rating {
    const fn from_id3v2(rating: u8) -> Self {
        match rating {
            0..=51 => Rating::Awful,
            52..=102 => Rating::Bad,
            103..=153 => Rating::Ok,
            154..=204 => Rating::Good,
            205..=255 => Rating::Amazing,
        }
    }

    const fn into_id3v2(&self) -> u8 {
        match self {
            Rating::Awful => 50,
            Rating::Bad => 100,
            Rating::Ok => 150,
            Rating::Good => 200,
            Rating::Amazing => 250,
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
            Rating::Awful => "20",
            Rating::Bad => "40",
            Rating::Ok => "60",
            Rating::Good => "80",
            Rating::Amazing => "100",
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
enum AudioFileFormat {
    Flac,
    Mp3,
    Opus,
}

fn traverse_audio_files(
    root: impl AsRef<Path>,
) -> impl Iterator<Item = (AudioFileFormat, PathBuf)> {
    walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|file| file.into_path())
        .filter_map(move |path| match path.extension() {
            Some(file_ext) => {
                if file_ext.eq_ignore_ascii_case("flac") {
                    Some((AudioFileFormat::Flac, path))
                } else if file_ext.eq_ignore_ascii_case("opus") {
                    Some((AudioFileFormat::Opus, path))
                } else if file_ext.eq_ignore_ascii_case("mp3") {
                    Some((AudioFileFormat::Mp3, path))
                } else {
                    None
                }
            }
            None => None,
        })
}
