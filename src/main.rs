mod app;
mod audio;
mod events;
mod jukebox;
mod pages;
mod terminal;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let music_dir = std::env::args().last().expect("expected dir path");

    let terminal = terminal::Terminal::init()?;
    let mut app = app::App::new(music_dir);
    let res = app.run(terminal);
    app.quit();
    terminal::Terminal::restore()?;
    res
}
