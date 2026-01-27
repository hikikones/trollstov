use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    jukebox::Jukebox,
    utils,
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

    pub fn on_enter(&self) {}

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        if jb.is_queue_empty() {
            const EMPTY_QUEUE: &str = "No tracks in the queue";
            buf.set_stringn(
                area.x + (area.width.saturating_sub(EMPTY_QUEUE.len() as u16)) / 2,
                area.y,
                EMPTY_QUEUE,
                EMPTY_QUEUE.len(),
                Style::new().fg(colors.neutral),
            );
            return;
        }

        self.scroll = utils::calculate_scroll(area.height, self.index, self.scroll);
        let mut line_area = Rect { height: 1, ..area };

        jb.queue_iter()
            .enumerate()
            .skip(self.scroll)
            .take(area.height as usize)
            .for_each(|(i, (_id, track))| {
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

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.queue_len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            _ => {}
        }
    }

    pub fn on_exit(&self) {}
}
