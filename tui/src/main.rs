mod app;
mod events;
mod pages;
mod terminal;
mod widgets;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let terminal = terminal::Terminal::init()?;

    // Create picker after entering alternate screen, but before reading terminal events
    let picker = ratatui_image::picker::Picker::from_query_stdio()?;

    let audio_device = jukebox::AudioDevice::new()?;
    let jukebox = jukebox::Jukebox::new(audio_device, args.dir);

    let mut app = app::App::new(jukebox, picker, args.mpris);
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

/// A simple music player for the terminal.
#[derive(Debug, clap::Parser)]
#[command(version, verbatim_doc_comment)]
struct Args {
    /// The directory for your music.
    #[arg(value_name = "MUSIC_DIR", value_hint = clap::ValueHint::DirPath, verbatim_doc_comment)]
    dir: std::path::PathBuf,

    /// Add media controls through the Media Player Remote Interfacing Specification (MPRIS),
    /// allowing music control with media keys and visually in your desktop environment.
    #[clap(long, action, verbatim_doc_comment)]
    mpris: bool,
}
