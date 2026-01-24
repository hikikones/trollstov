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
        Line::raw("TODO").centered().render(area, buf);
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                // self.index =
                //     usize::min(self.index + 1, self.search_results.len().saturating_sub(1));
                // self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                // self.index = self.index.saturating_sub(1);
                // self.events.send(AppEvent::Render);
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
