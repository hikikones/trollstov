mod app;
mod audio;
mod terminal;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let audio = audio::AudioPlayback::new()?;
    let music_dir = std::env::args().last().expect("expected dir path");
    let db = audio::Database::new(music_dir);

    let terminal = terminal::Terminal::init()?;
    let app = app::App::new(db, audio);
    let res = app.run(terminal);
    terminal::Terminal::restore()?;
    res
}
