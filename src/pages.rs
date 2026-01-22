mod logs;
mod playing;
mod search;
mod tracks;

pub use logs::*;
pub use playing::*;
pub use search::*;
pub use tracks::*;

use crate::events::EventSender;

pub struct Pages {
    pub tracks: TracksPage,
    pub search: SearchPage,
    pub now_playing: NowPlayingPage,
    pub logs: LogsPage,
}

impl Pages {
    pub fn new(picker: ratatui_image::picker::Picker, events: EventSender) -> Self {
        Self {
            tracks: TracksPage::new(events.clone()),
            search: SearchPage::new(events.clone()),
            now_playing: NowPlayingPage::new(picker),
            logs: LogsPage::new(events),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    #[default]
    Tracks,
    Search,
    NowPlaying,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks => Self::Search,
            Self::Search => Self::NowPlaying,
            Self::NowPlaying => Self::Logs,
            Self::Logs => Self::Tracks,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Logs,
            Self::Search => Self::Tracks,
            Self::NowPlaying => Self::Search,
            Self::Logs => Self::NowPlaying,
        }
    }
}
