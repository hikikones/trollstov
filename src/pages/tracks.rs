use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    app::Colors,
    audio::AudioRating,
    events::{AppEvent, EventHandler},
    jukebox::Jukebox,
};

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
        for (label, width, spacing) in [
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
            Span::raw(label).render(col, buf);
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

        // let mut x = table_area.x;
        let mut y = table_area.y;

        let current = jb.current();
        jb.iter()
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, (id, track))| {
                let mut line = String::new();

                for (text, width, spacing) in [
                    (track.title(), info_width, spacing),
                    (track.artist(), info_width, spacing),
                    (track.album(), info_width, spacing),
                    (track.duration_display(), time_width, spacing),
                    (track.rating_display(), rating_width, 0),
                ] {
                    let max_text_width = width.saturating_sub(spacing);
                    let text_width = text.width() as u16;
                    if text_width <= max_text_width {
                        // Push text and fill in remaining spaces
                        line.push_str(text);
                        for _ in 0..max_text_width.saturating_sub(text_width) + spacing {
                            line.push(' ');
                        }
                    } else {
                        let mut w = 0;
                        for g in text.graphemes(true) {
                            let gw = g.width() as u16;
                            if w + gw <= max_text_width {
                                //push
                                line.push_str(g);
                                w += gw;
                            } else {
                                //done, fill in remaining
                                for _ in 0..max_text_width.saturating_sub(w) + spacing {
                                    line.push(' ');
                                }
                                break;
                            }
                        }
                    }

                    // let col = Rect {
                    //     width: width.saturating_sub(spacing),
                    //     height: 1,
                    //     x,
                    //     y,
                    // };

                    // let mut style = Style::new();
                    // if self.index == i {
                    //     style.fg = Some(colors.accent);
                    // }
                    // if let Some(current) = current
                    //     && current == id
                    // {
                    //     style.add_modifier.insert(Modifier::BOLD);
                    // }

                    // Span::styled(text, style).render(col, buf);
                    // x += width;
                }
                // x = table_area.x;
                // y += 1;

                // let mut style = Style::new();
                // if self.index == i {
                //     style.fg = Some(colors.accent);
                // }
                // if let Some(current) = current
                //     && current == id
                // {
                //     style.add_modifier.insert(Modifier::BOLD);
                // }

                let r = Rect {
                    height: 1,
                    // y: table_area.y + i as u16,
                    y,
                    ..table_area
                };

                let mut style = Style::new();
                if self.index == i {
                    style.bg = Some(colors.accent);
                    style.fg = Some(colors.on_accent);
                }
                if let Some(current) = current
                    && current == id
                {
                    style.add_modifier.insert(Modifier::BOLD);
                }

                Span::styled(&line, style).render(r, buf);
                line.clear();

                y += 1;
            });
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        events: &EventHandler,
        jb: &mut Jukebox,
    ) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.len().saturating_sub(1));
                events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                let id = jb.get_key_from_index(self.index).unwrap();
                let _ = jb.play(id);
                events.send(AppEvent::Render);
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    let track = jb.values_mut().nth(self.index).unwrap();
                    track.set_rating(rating).unwrap();
                    events.send(AppEvent::Render);
                }
                's' => {
                    jb.sort(jb.get_sort().next());
                    events.send(AppEvent::Render);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
