mod app;
mod colors;
mod events;
mod pages;
mod settings;
mod terminal;
mod widgets;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let terminal = terminal::Terminal::init()?;

    // Create picker after entering alternate screen, but before reading terminal events
    let picker = ratatui_image::picker::Picker::from_query_stdio()?;

    let audio_device = jukebox::AudioDevice::new()?;
    let jukebox = jukebox::Jukebox::new(audio_device, args.dir);
    let colors = colors::Colors::new()
        .accent(args.accent_color)
        .on_accent(args.on_accent_color)
        .neutral(args.neutral_color)
        .on_neutral(args.on_neutral_color);

    let mut app = app::App::new(jukebox, colors, picker, args.mpris);
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
    long_about = "
A music player for the terminal that is built around a simple idea: your music is the database. \
It treats your audio files — and their metadata — as the single source of truth. \
Your filesystem is the index, and your tags are the schema. \
Simply back up your music directory and you have backed up everything. \
Ratings are part of the metadata, so your favorite songs are always just a few keystrokes away.

The application takes one mandatory argument, which is the path to your music directory. \
In addition, it comes with a few optional arguments that is mostly colors \
you define by name, hex code or indexed value.

See ANSI colors for options: https://en.wikipedia.org/wiki/ANSI_escape_code#Colors

EXAMPLES:
    # Run using mpris with different accent colors:
    trollstov --mpris /path/to/my/music --accent-color cyan --on-accent-color \"#000000\""
)]
struct Args {
    /// The directory for your music.
    #[arg(value_name = "MUSIC_DIR", value_hint = clap::ValueHint::DirPath)]
    dir: std::path::PathBuf,

    /// Try to establish media controls through the Media Player Remote Interfacing Specification (MPRIS),
    /// allowing music control with media keys and visually in your desktop environment.
    #[clap(long, action)]
    mpris: bool,

    /// The accent color of the application.
    #[clap(long, value_name = "COLOR")]
    accent_color: Option<ratatui::style::Color>,

    /// The color on top of an accent.
    #[clap(long, value_name = "COLOR")]
    on_accent_color: Option<ratatui::style::Color>,

    /// The neutral color of the application.
    #[clap(long, value_name = "COLOR")]
    neutral_color: Option<ratatui::style::Color>,

    /// The color on top of a neutral.
    #[clap(long, value_name = "COLOR")]
    on_neutral_color: Option<ratatui::style::Color>,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
