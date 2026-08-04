use std::path::{Path, PathBuf};

use shared::terminal::{Terminal, TerminalInfo};
use trollink::{app::App, database::Database, settings::Settings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let config_dir = get_config_dir();

    #[cfg(not(debug_assertions))]
    let database = open_database(args.database, config_dir.as_deref())?;
    #[cfg(debug_assertions)]
    let database = open_dev_database(args.database, config_dir.as_deref())?;

    let term_info = query_term_info()?;
    let settings = read_settings(args.settings, args.assets, config_dir.as_deref(), term_info)?;

    let mut terminal = Terminal::init()?;

    let mut app = App::new(database, settings);
    let res = app.run(&mut terminal);
    app.quit()?;

    terminal.restore()?;

    res
}

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = CLAP_STYLING)]
struct Args {
    /// Optional path for your database file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.db", value_hint = clap::ValueHint::FilePath)]
    database: Option<PathBuf>,

    /// Optional path for your assets directory. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    assets: Option<PathBuf>,

    /// Optional path for your settings file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
    settings: Option<PathBuf>,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

fn query_term_info() -> Result<shared::terminal::TerminalInfo, String> {
    shared::terminal::TerminalInfo::query()
        .map_err(|err| format!("Failed to get terminal info: {}", err))
}

fn open_database(path: Option<PathBuf>, config_dir: Option<&Path>) -> Result<Database, String> {
    let Some(file) = path.or(config_dir.map(|dir| dir.join("database.db"))) else {
        return Err("Failed to get a default database file path from the operating system")?;
    };

    ensure_path_exists(&file, true)
        .map_err(|err| format!("Failed to create path for '{}': {}", file.display(), err))?;

    Database::open(&file)
        .map_err(|err| format!("Failed to open database '{}': {}", file.display(), err))
}

#[cfg(debug_assertions)]
fn open_dev_database(path: Option<PathBuf>, config_dir: Option<&Path>) -> Result<Database, String> {
    if path.is_some() {
        open_database(path, config_dir)
    } else {
        Database::open_in_memory()
            .map_err(|err| format!("Failed to open database in memory: {}", err))
    }
}

fn read_settings(
    config_file: Option<PathBuf>,
    assets_dir: Option<PathBuf>,
    config_dir: Option<&Path>,
    info: TerminalInfo,
) -> Result<Settings, String> {
    let Some(config_file) = config_file.or(config_dir.map(|dir| dir.join("settings.toml"))) else {
        return Err("Failed to get a default settings file path from the operating system")?;
    };

    let Some(assets_dir) = assets_dir.or(config_dir.map(|dir| dir.join("assets"))) else {
        return Err("Failed to get a default assets directory from the operating system")?;
    };

    ensure_path_exists(&config_file, true).map_err(|err| {
        format!(
            "Failed to create path for '{}': {}",
            config_file.display(),
            err
        )
    })?;

    ensure_path_exists(&assets_dir, false).map_err(|err| {
        format!(
            "Failed to create path for '{}': {}",
            assets_dir.display(),
            err
        )
    })?;

    if !config_file.exists() {
        return Ok(Settings::new(config_file, assets_dir, info));
    }

    Settings::read(config_file, assets_dir, info)
}

fn ensure_path_exists(path: &Path, is_file: bool) -> std::io::Result<()> {
    let dir = if is_file {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };

    std::fs::create_dir_all(dir)
}

fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from(
        trollink::APP_QUALIFIER,
        trollink::APP_ORGANIZATION,
        trollink::APP_NAME,
    )
    .map(|dirs| dirs.config_dir().to_path_buf())
}
