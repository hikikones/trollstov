mod logs;
mod playing;
mod search;
mod tracks;

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
    pub fn new(colors: &Colors) -> Self {
        Self {
            tracks: TracksPage::new(),
            playing: PlayingPage::new(),
            search: SearchPage::new(colors),
            logs: LogsPage::new(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    #[default]
    Tracks,
    NowPlaying,
    Search,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks => Self::NowPlaying,
            Self::NowPlaying => Self::Search,
            Self::Search => Self::Logs,
            Self::Logs => Self::Tracks,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Logs,
            Self::NowPlaying => Self::Tracks,
            Self::Search => Self::NowPlaying,
            Self::Logs => Self::Search,
        }
    }
}
