fn main() {
    let dir = std::env::args().last().expect("expected dir path");

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".opus"))
                .unwrap_or(false)
        })
    {
        println!("{}", entry.path().display());
    }
}
