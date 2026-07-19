use lazycard::{app::App, database::Database};
use shared::terminal::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let palette = query_colors()?;
    let cell_size = query_cell_size()?;

    #[cfg(debug_assertions)]
    let database = open_dev_database(args.database)?;
    #[cfg(not(debug_assertions))]
    let database = open_database(args.database)?;

    let assets_dir = get_or_create_assets_dir(args.assets)?;

    let mut terminal = Terminal::init()?;

    let mut app = App::new(database, args.settings, assets_dir, cell_size, palette);
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
    database: Option<std::path::PathBuf>,

    /// Optional path for your assets directory. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    assets: Option<std::path::PathBuf>,

    /// Optional path for your settings file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
    settings: Option<std::path::PathBuf>,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

fn query_colors() -> Result<shared::terminal::TerminalPalette, String> {
    shared::terminal::TerminalPalette::query()
        .map_err(|err| format!("Failed to get terminal colors: {}", err))
}

fn query_cell_size() -> Result<shared::terminal::TerminalCellSize, String> {
    shared::terminal::TerminalCellSize::query()
        .map_err(|err| format!("Failed to get terminal cell size: {}", err))
}

fn open_database(path: Option<std::path::PathBuf>) -> Result<Database, String> {
    fn get_default_database_file() -> Option<std::path::PathBuf> {
        const FILENAME: &str = "database.db";
        directories::ProjectDirs::from(
            lazycard::APP_QUALIFIER,
            lazycard::APP_ORGANIZATION,
            lazycard::APP_NAME,
        )
        .map(|project_dirs| project_dirs.config_dir().join(FILENAME))
    }

    let file = match path {
        Some(path) => path,
        None => match get_default_database_file() {
            Some(path) => path,
            None => {
                return Err(
                    "Failed to get a default database file path from the operating system",
                )?;
            }
        },
    };

    Database::open(&file).map_err(|err| {
        format!(
            "Failed to open database \"{}\" due to {}",
            file.display(),
            err,
        )
    })
}

#[cfg(debug_assertions)]
fn open_dev_database(path: Option<std::path::PathBuf>) -> Result<Database, String> {
    if path.is_some() {
        open_database(path)
    } else {
        Database::open_in_memory()
            .map_err(|err| format!("Failed to open database in memory due to {}", err))
    }
}

fn get_or_create_assets_dir(
    path: Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    fn get_default_assets_dir() -> Option<std::path::PathBuf> {
        const DIRNAME: &str = "assets";
        directories::ProjectDirs::from(
            lazycard::APP_QUALIFIER,
            lazycard::APP_ORGANIZATION,
            lazycard::APP_NAME,
        )
        .map(|project_dirs| project_dirs.config_dir().join(DIRNAME))
    }

    let dir = match path {
        Some(path) => path,
        None => match get_default_assets_dir() {
            Some(path) => path,
            None => {
                return Err("Failed to get a default assets directory from the operating system")?;
            }
        },
    };

    // Make sure assets dir is created
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|err| {
            format!(
                "Failed to create assets directory at \"{}\" due to {}",
                dir.display(),
                err
            )
        })?;
    }
    // If already exists, make sure it is actually a dir
    else if !dir.is_dir() {
        return Err(format!(
            "Assets directory \"{}\" is not a directory",
            dir.display()
        ))?;
    }

    Ok(dir)
}
