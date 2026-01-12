use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use crate::{
    app::{Action, Colors},
    audio::{AudioPicture, AudioRating},
    jukebox::{Jukebox, TrackId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Tracks,
    NowPlaying,
}

pub struct Pages {
    pub tracks: TracksPage,
    pub now_playing: NowPlayingPage,
}

impl Pages {
    pub fn new() -> Self {
        Self {
            tracks: TracksPage::new(),
            now_playing: NowPlayingPage::new(),
        }
    }
}

pub struct TracksPage {
    index: usize,
    scroll: usize,
}

impl TracksPage {
    pub fn new() -> Self {
        Self {
            index: 0,
            scroll: 0,
        }
    }

    pub fn on_enter(&mut self, jb: &Jukebox) {
        // todo
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        colors: &Colors,
        menu: &mut Line,
    ) {
        let spacing = 2;
        let time_width = 6 + spacing;
        let rating_width = 6;
        let remaining_width = area.width.saturating_sub(time_width + rating_width);
        let info_width = remaining_width / 3;

        let [header_area, table_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

        let mut x = header_area.x;
        for (header, width, spacing) in [
            ("Title", info_width, spacing),
            ("Artist", info_width, spacing),
            ("Album", info_width, spacing),
            ("Time", time_width, spacing),
            ("Rating", rating_width, 0),
        ] {
            let col = Rect {
                width: width.saturating_sub(spacing),
                height: 1,
                x,
                y: header_area.y,
            };
            Span::raw(header).render(col, buf);
            x += width;
        }

        let height = table_area.height as usize;
        if self.index > self.scroll {
            let height_diff = self.index - self.scroll;
            let height = height.saturating_sub(1);
            if height_diff > height {
                self.scroll += height_diff - height;
            }
        } else if self.scroll > self.index {
            let height_diff = self.scroll - self.index;
            self.scroll -= height_diff;
        }

        let mut x = table_area.x;
        let mut y = table_area.y;

        let current = jb.current();
        jb.iter()
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, (id, track))| {
                for (text, width, spacing) in [
                    (track.title(), info_width, spacing),
                    (track.artist(), info_width, spacing),
                    (track.album(), info_width, spacing),
                    (track.duration_display(), time_width, spacing),
                    (track.rating_display(), rating_width, 0),
                ] {
                    let col = Rect {
                        width: width.saturating_sub(spacing),
                        height: 1,
                        x,
                        y,
                    };

                    let mut style = Style::new();
                    if self.index == i {
                        style.fg = Some(colors.accent);
                    }
                    if let Some(current) = current
                        && current == id
                    {
                        style.add_modifier.insert(Modifier::BOLD);
                    }

                    Span::styled(text, style).render(col, buf);
                    x += width;
                }
                x = table_area.x;
                y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) -> Action {
        match key {
            KeyCode::Esc => Action::Quit,
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.len().saturating_sub(1));
                Action::Render
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                Action::Render
            }
            KeyCode::Enter => {
                let id = jb.get_key_from_index(self.index).unwrap();
                let _ = jb.play(id);
                Action::Render
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let rating: AudioRating = AudioRating::from_char(c).unwrap();
                    let track = jb.values_mut().nth(self.index).unwrap();
                    track.set_rating(rating).unwrap();
                    Action::Render
                }
                's' => {
                    jb.sort(jb.get_sort().next());
                    Action::Render
                }
                _ => Action::None,
            },
            _ => Action::None,
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}

pub struct NowPlayingPage {
    current: Option<TrackId>,
    image_protocol: Option<StatefulProtocol>,
}

impl NowPlayingPage {
    pub fn new() -> Self {
        Self {
            current: None,
            image_protocol: None,
        }
    }

    pub fn on_enter(&mut self, jb: &Jukebox) {
        // todo
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox) {
        if self.current != jb.current() {
            self.current = jb.current();

            // song change, update image
            match jb.current() {
                Some(tid) => {
                    // load new image
                    let track = jb.get(tid).unwrap();
                    let picture = AudioPicture::read(track.path()).unwrap();
                    match picture.bytes() {
                        Some(bytes) => {
                            let picker = Picker::from_query_stdio().unwrap();
                            let dyn_img = image::load_from_memory(bytes).unwrap();
                            let image = picker.new_resize_protocol(dyn_img);
                            self.image_protocol = Some(image);
                        }
                        None => {
                            self.image_protocol = None;
                        }
                    }
                }
                None => {
                    // remove image
                    self.image_protocol = None;
                }
            }
        }

        // Show currently playing, image or not
        match self.current {
            Some(_) => match &mut self.image_protocol {
                Some(image) => {
                    StatefulImage::default().render(area, buf, image);
                }
                None => {
                    Line::raw("NO IMAGE").centered().render(area, buf);
                }
            },
            None => {
                Line::raw("NO TRACK CURRENTLY PLAYING")
                    .centered()
                    .render(area, buf);
            }
        }
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        // todo
        Action::None
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
