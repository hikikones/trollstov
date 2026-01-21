mod logs;
mod playing;
mod tracks;

pub use logs::*;
pub use playing::*;
pub use tracks::*;

use crate::events::EventSender;

pub struct Pages {
    pub tracks: TracksPage,
    pub now_playing: NowPlayingPage,
    pub logs: LogsPage,
}

impl Pages {
    pub fn new(picker: ratatui_image::picker::Picker, events: EventSender) -> Self {
        Self {
            tracks: TracksPage::new(events.clone()),
            now_playing: NowPlayingPage::new(picker),
            logs: LogsPage::new(events),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    #[default]
    Tracks,
    NowPlaying,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks => Self::NowPlaying,
            Self::NowPlaying => Self::Logs,
            Self::Logs => Self::Tracks,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Logs,
            Self::NowPlaying => Self::Tracks,
            Self::Logs => Self::NowPlaying,
        }
    }
}
