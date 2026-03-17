mod app;
mod events;
mod pages;
mod settings;
mod symbols;
mod terminal;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let terminal = terminal::Terminal::init()?;

    // Create picker after entering alternate screen, but before reading terminal events
    let picker = ratatui_image::picker::Picker::from_query_stdio()?;

    let player = jukebox::AudioPlayer::new()?;
    let jukebox = jukebox::Jukebox::new(player);
    let database = database::Database::new(args.dir);

    let mut app = app::App::new(database, jukebox, picker, args.mpris);
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

#[derive(Debug, clap::Parser)]
#[command(
    styles = CLAP_STYLING,
    version,
    about,
)]
struct Args {
    /// The directory for your music.
    #[arg(value_name = "MUSIC_DIR", value_hint = clap::ValueHint::DirPath)]
    dir: std::path::PathBuf,

    /// Try to establish media controls through the Media Player Remote Interfacing Specification (MPRIS),
    /// allowing music control with media keys and visually in your desktop environment.
    #[clap(long, action)]
    mpris: bool,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
