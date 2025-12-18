fn main() {
    let dir = std::env::args().last().expect("expected dir path");

    for path in traverse_audio_files(dir) {
        println!("{}", path.display());
    }
}

fn traverse_audio_files(
    root: impl AsRef<std::path::Path>,
) -> impl Iterator<Item = std::path::PathBuf> {
    const AUDIO_EXTS: &[&str] = &[
        "mp3", "wav", "flac", "ogg", "opus", "aac", "m4a", "wma", "alac", "aiff",
    ];

    walkdir::WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|file| file.into_path())
        .filter(|path| {
            path.extension()
                .map(|file_ext| {
                    AUDIO_EXTS
                        .iter()
                        .any(|audio_ext| file_ext.eq_ignore_ascii_case(audio_ext))
                })
                .unwrap_or(false)
        })
}
