mod logs;
mod playing;
mod queue;
mod search;
mod tracks;

pub use logs::*;
pub use playing::*;
pub use queue::*;
pub use search::*;
pub use tracks::*;

use crate::{app::Colors, events::EventSender};

pub struct Pages {
    pub tracks: TracksPage,
    pub now_playing: NowPlayingPage,
    pub queue: QueuePage,
    pub search: SearchPage,
    pub logs: LogsPage,
}

impl Pages {
    pub fn new(
        picker: ratatui_image::picker::Picker,
        events: EventSender,
        colors: &Colors,
    ) -> Self {
        Self {
            tracks: TracksPage::new(events.clone()),
            now_playing: NowPlayingPage::new(picker),
            queue: QueuePage::new(events.clone()),
            search: SearchPage::new(colors, events.clone()),
            logs: LogsPage::new(events),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    #[default]
    Tracks,
    NowPlaying,
    Queue,
    Search,
    Logs,
}

impl Route {
    pub const fn next(self) -> Self {
        match self {
            Self::Tracks => Self::NowPlaying,
            Self::NowPlaying => Self::Queue,
            Self::Queue => Self::Search,
            Self::Search => Self::Logs,
            Self::Logs => Self::Tracks,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Tracks => Self::Logs,
            Self::NowPlaying => Self::Tracks,
            Self::Queue => Self::NowPlaying,
            Self::Search => Self::Queue,
            Self::Logs => Self::Search,
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Tracks => "Tracks",
            Self::NowPlaying => "Now Playing",
            Self::Queue => "Queue",
            Self::Search => "Search",
            Self::Logs => "Logs",
        }
    }
}
