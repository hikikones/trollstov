use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::app::Colors;

pub struct LogsPage {
    logs: Vec<Log>,
}

impl LogsPage {
    pub const fn new() -> Self {
        Self { logs: Vec::new() }
    }

    pub fn on_enter(&self) {
        // todo
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, colors: &Colors) {
        Line::raw("TODO").centered().render(area, buf);
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) {}

    pub fn on_exit(&mut self) {
        // todo
    }
}

pub struct Log {
    message: String,
    level: LogLevel,
}

impl Log {
    pub fn new(message: impl Into<String>, level: LogLevel) -> Self {
        Self {
            message: message.into(),
            level,
        }
    }
}

pub enum LogLevel {
    Info,
    Warning,
    Error,
}
