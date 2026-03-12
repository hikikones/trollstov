mod logs;
mod playing;
mod search;
mod settings;
mod tracks;

pub use logs::*;
pub use playing::*;
pub use search::*;
pub use settings::*;
pub use tracks::*;

pub struct Pages {
    pub tracks: TracksPage,
    pub playing: PlayingPage,
    pub search: SearchPage,
    pub settings: SettingsPage,
    pub logs: LogsPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Tracks(Option<database::TrackId>),
    NowPlaying,
    Search,
    Settings,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks(_) => Self::NowPlaying,
            Self::NowPlaying => Self::Search,
            Self::Search => Self::Settings,
            Self::Settings => Self::Logs,
            Self::Logs => Self::Tracks(None),
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks(_) => Self::Logs,
            Self::NowPlaying => Self::Tracks(None),
            Self::Search => Self::NowPlaying,
            Self::Settings => Self::Search,
            Self::Logs => Self::Settings,
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::Tracks(None)
    }
}
