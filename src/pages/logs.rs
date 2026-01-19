use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::app::Colors;

pub struct LogsPage {
    logs: Vec<Log>,
    queue: Vec<Log>,
}

impl LogsPage {
    pub const fn new() -> Self {
        Self {
            logs: Vec::new(),
            queue: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, log: Log) {
        self.queue.push(log);
    }

    pub const fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn on_enter(&mut self) {
        self.logs.extend(self.queue.drain(..));
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, colors: &Colors) {
        let mut line_area = Rect { height: 1, ..area };
        let mut line = Line::default();

        for log in self.logs.iter() {
            let (label, style) = match log.level {
                LogLevel::Info => ("Info", Style::new().fg(Color::Green)),
                LogLevel::Warning => ("Warning", Style::new().fg(Color::Yellow)),
                LogLevel::Error => ("Error", Style::new().fg(Color::Red)),
            };
            line.push_span(Span::styled(label, style));
            line.push_span(Span::raw(" "));
            line.push_span(log.message.as_str());
            (&line).render(line_area, buf);

            line.spans.clear();
            line_area.y += 1;
        }
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
