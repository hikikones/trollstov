use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    editor::TextInput,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, Track, TrackId},
};

pub struct SearchPage {
    index: usize,
    scroll: usize,
    input: TextInput,
    tracks: Vec<TrackId>,
    events: EventSender,
    line_buffer: String,
}

impl SearchPage {
    pub fn new(events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            input: TextInput::new().with_placeholder("search..."),
            tracks: Vec::new(),
            events,
            line_buffer: String::new(),
        }
    }

    pub fn on_enter(&mut self) {
        // todo
        for _ in 0..100 {
            self.tracks.push(TrackId::default());
        }
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        colors: &Colors,
        menu: &mut Line,
    ) {
        self.input.render(
            area.centered_horizontally(Constraint::Percentage(60)),
            buf,
            colors,
        );

        let area = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(2),
            ..area
        };

        let height = area.height as usize;
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

        let mut buffer = itoa::Buffer::new();
        let mut line_area = Rect { height: 1, ..area };

        self.tracks
            .iter()
            .copied()
            .filter_map(|id| jb.get(id))
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, track)| {
                self.line_buffer.extend([
                    buffer.format(i + 1),
                    " ",
                    track.artist(),
                    " - ",
                    track.album(),
                    " - ",
                    track.title(),
                ]);

                let style = if self.index == i {
                    Style::new().bg(colors.accent).fg(colors.on_accent)
                } else {
                    Style::new()
                };

                Span::styled(&self.line_buffer, style).render(line_area, buf);

                self.line_buffer.clear();
                line_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, self.tracks.len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                // todo
                // let id = jb.get_key_from_index(self.index).unwrap();
                // jb.play(id);
            }
            _ => {
                if self.input.input(key, modifiers) {
                    self.events.send(AppEvent::Render);
                }
            }
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
