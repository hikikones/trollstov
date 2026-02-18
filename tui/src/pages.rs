mod logs;
mod playing;
mod search;
mod tracks;

use jukebox::TrackId;
pub use logs::*;
pub use playing::*;
pub use search::*;
pub use tracks::*;

use crate::app::Colors;

pub struct Pages {
    pub tracks: TracksPage,
    pub playing: PlayingPage,
    pub search: SearchPage,
    pub logs: LogsPage,
}

impl Pages {
    pub const fn new(colors: &Colors) -> Self {
        Self {
            tracks: TracksPage::new(),
            playing: PlayingPage::new(),
            search: SearchPage::new(colors),
            logs: LogsPage::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Tracks(Option<TrackId>),
    NowPlaying,
    Search,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks(_) => Self::NowPlaying,
            Self::NowPlaying => Self::Search,
            Self::Search => Self::Logs,
            Self::Logs => Self::Tracks(None),
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks(_) => Self::Logs,
            Self::NowPlaying => Self::Tracks(None),
            Self::Search => Self::NowPlaying,
            Self::Logs => Self::Search,
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::Tracks(None)
    }
}
