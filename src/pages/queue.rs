use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, TrackId},
};

pub struct QueuePage {
    index: usize,
    scroll: usize,
    events: EventSender,
    buffer: String,
}

impl QueuePage {
    pub fn new(events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            events,
            buffer: String::new(),
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

        let mut line_area = Rect { height: 1, ..area };

        jb.queue_iter()
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, (id, track))| {
                self.buffer
                    .extend([track.title(), " ", track.artist(), " ", track.album()]);

                let mut style = Style::new();
                if self.index == i {
                    style.bg = Some(colors.accent);
                    style.fg = Some(colors.on_accent);
                }

                Line::styled(&self.buffer, style).render(line_area, buf);

                self.buffer.clear();
                line_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.queue_len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                // if let Some((id, _)) = self.search_results.get(self.index).copied() {
                //     jb.play(id);
                // }
            }
            _ => {}
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
