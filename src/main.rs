use std::borrow::Cow;

use lofty::{
    file::AudioFile,
    id3::v2::{Frame, FrameId, Id3v2Tag},
    tag::Accessor,
};

fn main() {
    let dir = std::env::args().last().expect("expected dir path");

    let mut count = 0;

    for (audio_format, path) in traverse_audio_files(dir) {
        println!(
            "Audio File Format: {:?}\nPath: {}",
            audio_format,
            path.display()
        );

        let mut file = std::fs::File::open(&path).unwrap();

        let track = match audio_format {
            AudioFileFormat::Mp3 => {
                let mp3 =
                    lofty::mpeg::MpegFile::read_from(&mut file, lofty::config::ParseOptions::new())
                        .unwrap();
                Track::from_id3v2(mp3.id3v2().unwrap())
            }
            AudioFileFormat::Flac => {
                let flac =
                    lofty::flac::FlacFile::read_from(&mut file, lofty::config::ParseOptions::new())
                        .unwrap();
                Track::from_vorbis_comments(flac.vorbis_comments().unwrap())
            }
            AudioFileFormat::Opus => {
                let opus =
                    lofty::ogg::OpusFile::read_from(&mut file, lofty::config::ParseOptions::new())
                        .unwrap();
                Track::from_vorbis_comments(opus.vorbis_comments())
            }
        };

        println!("{track:?}");

        if count == 20 {
            break;
        } else {
            count += 1;
        }
    }
}

#[derive(Debug)]
struct Track {
    title: String,
    artist: String,
    album: String,
    rating: Option<Rating>,
}

impl Track {
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

#[derive(Debug)]
enum Rating {
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
    root: impl AsRef<std::path::Path>,
) -> impl Iterator<Item = (AudioFileFormat, std::path::PathBuf)> {
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
