use std::{
    fs::File,
    path::{Path, PathBuf},
};

use lofty::{config::ParseOptions, file::AudioFile, flac::FlacFile, mpeg::MpegFile, ogg::OpusFile};

use crate::audio::*;

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
            .collect();

        Self { tracks }
    }
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
