use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
};

use lofty::{
    config::ParseOptions,
    file::AudioFile,
    flac::FlacFile,
    id3::v2::{Frame, FrameId, Id3v2Tag},
    mpeg::MpegFile,
    ogg::OpusFile,
    tag::Accessor,
};

#[derive(Debug)]
pub struct Database {
    tracks: Vec<Track>,
}

impl Database {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let tracks = traverse_audio_files(dir)
            .take(20)
            .map(|(audio_format, path)| {
                let mut file = File::open(&path).unwrap();

                let metadata = match audio_format {
                    AudioFileFormat::Mp3 => {
                        let mp3 = MpegFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        Metadata::from_id3v2(mp3.id3v2().unwrap())
                    }
                    AudioFileFormat::Flac => {
                        let flac = FlacFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        Metadata::from_vorbis_comments(flac.vorbis_comments().unwrap())
                    }
                    AudioFileFormat::Opus => {
                        let opus = OpusFile::read_from(&mut file, ParseOptions::new()).unwrap();
                        Metadata::from_vorbis_comments(opus.vorbis_comments())
                    }
                };

                Track {
                    metadata,
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
    path: PathBuf,
    audio_format: AudioFileFormat,
}

impl Track {
    pub fn title(&self) -> &str {
        &self.metadata.title
    }

    pub fn artist(&self) -> &str {
        &self.metadata.artist
    }

    pub fn album(&self) -> &str {
        &self.metadata.album
    }

    pub fn rating(&self) -> Option<Rating> {
        self.metadata.rating
    }

    pub fn set_rating(&mut self, rating: Option<Rating>) {
        self.metadata.rating = rating;
    }

    pub fn path(&self) -> &Path {
        &self.path
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

    fn from_vorbis_comments(metadata: &lofty::ogg::VorbisComments) -> Self {
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
}

#[derive(Debug, Clone, Copy)]
pub enum Rating {
    Awful,
    Bad,
    Ok,
    Good,
    Amazing,
}

impl Rating {
    fn from_id3v2(rating: u8) -> Self {
        match rating {
            0..=51 => Rating::Awful,
            52..=102 => Rating::Bad,
            103..=153 => Rating::Ok,
            154..=204 => Rating::Good,
            205..=255 => Rating::Amazing,
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
