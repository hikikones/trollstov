use lofty::file::AudioFile;

fn main() {
    let dir = std::env::args().last().expect("expected dir path");

    for (audio_format, path) in traverse_audio_files(dir) {
        println!(
            "Audio File Format: {:?}\nPath: {}",
            audio_format,
            path.display()
        );

        let mut file_content = std::fs::File::open(&path).unwrap();

        let track = match audio_format {
            AudioFileFormat::Flac => todo!(),
            AudioFileFormat::Mp3 => todo!(),
            AudioFileFormat::Opus => {
                let opus_file = lofty::ogg::OpusFile::read_from(
                    &mut file_content,
                    lofty::config::ParseOptions::new(),
                )
                .unwrap();

                let metadata = opus_file.vorbis_comments();
                Track {
                    title: metadata.get("TITLE").map(str::to_owned).unwrap_or_default(),
                    artist: metadata
                        .get("ARTIST")
                        .map(str::to_owned)
                        .unwrap_or_default(),
                    album: metadata.get("ALBUM").map(str::to_owned).unwrap_or_default(),
                    rating: metadata
                        .get("RATING")
                        .and_then(|s| Rating::try_from(s).ok()),
                }
            }
        };

        println!("{track:?}");

        break;
    }
}

#[derive(Debug)]
struct Track {
    title: String,
    artist: String,
    album: String,
    rating: Option<Rating>,
}

#[derive(Debug)]
enum Rating {
    Awful,
    Bad,
    Ok,
    Good,
    Amazing,
}

impl TryFrom<&str> for Rating {
    type Error = std::num::ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let num = value.parse::<u8>()?;
        let rating = match num {
            0..=20 => Self::Awful,
            21..=40 => Self::Bad,
            41..=60 => Self::Ok,
            61..=80 => Self::Good,
            81..=255 => Self::Amazing,
        };
        Ok(rating)
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
