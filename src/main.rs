mod app;
mod events;
mod jukebox;
mod pages;
mod terminal;
mod utils;
mod widgets;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let music_dir = std::env::args().last().expect("expected dir path");

    let terminal = terminal::Terminal::init()?;

    // Create picker after entering alternate screen, but before reading terminal events
    let picker = ratatui_image::picker::Picker::from_query_stdio()?;

    let events = events::EventHandler::new();
    let jukebox = jukebox::Jukebox::new(music_dir)?;

    let mut app = app::App::new(events, jukebox, picker);
    let res = app.run(terminal);
    app.quit();

    terminal::Terminal::restore()?;

    res
}
