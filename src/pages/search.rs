use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    jukebox::Jukebox,
};

pub struct SearchPage {
    index: usize,
    scroll: usize,
    line_buffer: String,
    events: EventSender,
}

impl SearchPage {
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
        // TODO: display each line as artist - album - title
        // no need for "table" here, just lines
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
                // todo
                // let id = jb.get_key_from_index(self.index).unwrap();
                // jb.play(id);
            }
            _ => {}
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}
