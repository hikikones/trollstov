use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;

use crate::{
    app::{Action, Colors},
    audio::{AudioPlayback, AudioRating},
    database::Database,
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
    current: usize,
    selected: Option<usize>,
    scroll: usize,
}

impl TracksPage {
    pub fn new() -> Self {
        Self {
            current: 0,
            selected: None,
            scroll: 0,
        }
    }

    pub fn on_enter(&mut self, db: &Database) {
        // todo
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
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
        if self.current > self.scroll {
            let height_diff = self.current - self.scroll;
            let height = height.saturating_sub(1);
            if height_diff > height {
                self.scroll += height_diff - height;
            }
        } else if self.scroll > self.current {
            let height_diff = self.scroll - self.current;
            self.scroll -= height_diff;
        }

        let mut x = table_area.x;
        let mut y = table_area.y;

        db.iter()
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, track)| {
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
                    if self.current == i {
                        style.fg = Some(colors.accent);
                    }
                    if let Some(selected) = self.selected
                        && selected == i
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

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        db: &mut Database,
        audio: &mut AudioPlayback,
    ) -> Action {
        match key {
            KeyCode::Esc => Action::Quit,
            KeyCode::Down => {
                self.current = usize::min(self.current + 1, db.len().saturating_sub(1));
                Action::Render
            }
            KeyCode::Up => {
                self.current = self.current.saturating_sub(1);
                Action::Render
            }
            KeyCode::Enter => {
                self.selected = Some(self.current);
                let track = db.iter().nth(self.current).unwrap();
                let _ = audio.play(track.path());
                Action::Render
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    let track = db.iter_mut().nth(self.current).unwrap();
                    track.set_rating(rating).unwrap();
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

pub struct NowPlayingPage {}

impl NowPlayingPage {
    pub fn new() -> Self {
        Self {}
    }

    pub fn on_enter(&mut self, db: &Database) {
        // todo
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer) {
        Line::raw("TODO").centered().render(area, buf);
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        // todo
        Action::None
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
