use lofty::{file::TaggedFileExt, tag::Accessor};

// TODO
enum Rating {
    Awful,
    Bad,
    Ok,
    Good,
    Amazing,
}

fn main() {
    let dir = std::env::args().last().expect("expected dir path");

    for (audio_format, path) in traverse_audio_files(dir) {
        println!(
            "Audio File Format: {:?}\nPath: {}",
            audio_format,
            path.display()
        );

        let mut tagged_file = lofty::read_from_path(path).unwrap();

        let tag = match tagged_file.primary_tag_mut() {
            Some(primary_tag) => primary_tag,
            None => tagged_file.first_tag_mut().expect("ERROR: No tags found!"),
        };

        println!("\n--- Tag Information ---");
        println!("Tag type: {:?}", tag.tag_type());
        println!("Title: {}", tag.title().as_deref().unwrap_or("None"));
        println!("Artist: {}", tag.artist().as_deref().unwrap_or("None"));
        println!("Album: {}", tag.album().as_deref().unwrap_or("None"));
        println!("Genre: {}", tag.genre().as_deref().unwrap_or("None"));

        break;
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
