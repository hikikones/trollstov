use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    app::Colors,
    audio::AudioRating,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, TrackSort},
};

pub struct TracksPage {
    index: usize,
    scroll: usize,
    line_buffer: String,
    events: EventSender,
}

impl TracksPage {
    pub fn new(events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            line_buffer: String::new(),
            events,
        }
    }

    pub fn on_enter(&mut self) {
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
        let rating_width = 7;
        let remaining_width = area.width.saturating_sub(time_width + rating_width);
        let info_width = remaining_width / 3;

        let [header_area, table_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(0)]).areas(area);

        // Render the header for the table
        let sort = jb.get_sort();
        let mut x = header_area.x;
        for (label, width, spacing) in [
            (
                if sort == TrackSort::TitleAscending {
                    "Title⌄"
                } else {
                    "Title"
                },
                info_width,
                spacing,
            ),
            (
                if sort == TrackSort::ArtistAscending {
                    "Artist⌄"
                } else {
                    "Artist"
                },
                info_width,
                spacing,
            ),
            (
                if sort == TrackSort::AlbumAscending {
                    "Album⌄"
                } else {
                    "Album"
                },
                info_width,
                spacing,
            ),
            (
                if sort == TrackSort::TimeAscending {
                    "Time⌄"
                } else {
                    "Time"
                },
                time_width,
                spacing,
            ),
            (
                if sort == TrackSort::RatingAscending {
                    "Rating⌄"
                } else {
                    "Rating"
                },
                rating_width,
                0,
            ),
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

        // Render the body for the table
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

        let mut row_area = Rect {
            height: 1,
            ..table_area
        };
        let current = jb.current();
        self.line_buffer.reserve(table_area.width as usize);

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
                    // Determine how much text we can fill for each column
                    let max_text_width = width.saturating_sub(spacing);
                    let text_width = text.width() as u16;
                    if text_width <= max_text_width {
                        // Text fits, fill in remaining with spaces
                        self.line_buffer.push_str(text);
                        for _ in 0..max_text_width.saturating_sub(text_width) + spacing {
                            self.line_buffer.push(' ');
                        }
                    } else {
                        // No fit, fill in what we can
                        let mut curr_width = 0;
                        for grapheme in text.graphemes(true) {
                            let grapheme_width = grapheme.width() as u16;
                            if curr_width + grapheme_width <= max_text_width {
                                // Keep pushing
                                self.line_buffer.push_str(grapheme);
                                curr_width += grapheme_width;
                            } else {
                                // Done, fill in remaining with spaces
                                for _ in 0..max_text_width.saturating_sub(curr_width) + spacing {
                                    self.line_buffer.push(' ');
                                }
                                break;
                            }
                        }
                    }
                }

                let mut style = Style::new();
                if self.index == i {
                    style.bg = Some(colors.accent);
                    style.fg = Some(colors.on_accent);
                }
                if current == Some(id) {
                    style.add_modifier.insert(Modifier::BOLD);
                }

                Span::styled(&self.line_buffer, style).render(row_area, buf);

                self.line_buffer.clear();
                row_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                let id = jb.get_key_from_index(self.index).unwrap();
                jb.play(id);
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let id = jb.get_key_from_index(self.index).unwrap();
                    let rating = AudioRating::from_char(c).unwrap();
                    jb.set_rating(id, rating);
                }
                's' => {
                    jb.sort(jb.get_sort().next());
                    self.events.send(AppEvent::Render);
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
