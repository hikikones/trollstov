pub mod app;
pub mod database;
pub mod events;
pub mod jukebox;
pub mod logo;
pub mod pages;
pub mod settings;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_QUALIFIER: &str = "org";
const APP_ORGANIZATION: &str = "hikikones";
