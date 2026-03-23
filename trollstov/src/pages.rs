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
    Settings,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks(_) => Self::NowPlaying,
            Self::NowPlaying => Self::Settings,
            Self::Settings => Self::Tracks(None),
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks(_) => Self::Settings,
            Self::NowPlaying => Self::Tracks(None),
            Self::Settings => Self::NowPlaying,
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::Tracks(None)
    }
}
