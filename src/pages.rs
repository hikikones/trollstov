mod playing;
mod tracks;

pub use playing::*;
pub use tracks::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    #[default]
    Tracks,
    NowPlaying,
}

pub struct Pages {
    pub tracks: TracksPage,
    pub now_playing: NowPlayingPage,
}

impl Pages {
    pub fn new(picker: ratatui_image::picker::Picker) -> Self {
        Self {
            tracks: TracksPage::new(),
            now_playing: NowPlayingPage::new(picker),
        }
    }
}
