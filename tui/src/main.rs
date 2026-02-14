mod app;
mod events;
mod pages;
mod terminal;
mod widgets;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let music_dir = std::env::args().nth(1).expect("music directory");

    let terminal = terminal::Terminal::init()?;

    // Create picker after entering alternate screen, but before reading terminal events
    let picker = ratatui_image::picker::Picker::from_query_stdio()?;

    let audio_device = jukebox::AudioDevice::new()?;
    let jukebox = jukebox::Jukebox::new(audio_device, music_dir);

    let mut app = app::App::new(jukebox, picker);
    let res = app.run(terminal);

    match terminal::Terminal::restore() {
        Ok(_) => {
            app.quit();
        }
        Err(err) => {
            app.quit();
            return Err(err)?;
        }
    }

    res
}
