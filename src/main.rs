mod app;
mod audio;
mod events;
mod jukebox;
mod pages;
mod terminal;
mod utils;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let music_dir = std::env::args().last().expect("expected dir path");

    let terminal = terminal::Terminal::init()?;
    let app = app::App::new(music_dir);
    let res = app.run(terminal);
    terminal::Terminal::restore()?;
    res
}
