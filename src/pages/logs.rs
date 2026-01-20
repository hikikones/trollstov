use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventHandler},
};

pub struct LogsPage {
    logs: Vec<Log>,
    queue: Vec<Log>,
    index: usize,
    vertical_scroll: usize,
    horizontal_scroll: usize,
}

impl LogsPage {
    pub const fn new() -> Self {
        Self {
            logs: Vec::new(),
            queue: Vec::new(),
            index: 0,
            vertical_scroll: 0,
            horizontal_scroll: 0,
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

        let height = area.height as usize;
        if self.index > self.vertical_scroll {
            let height_diff = self.index - self.vertical_scroll;
            let height = height.saturating_sub(1);
            if height_diff > height {
                self.vertical_scroll += height_diff - height;
            }
        } else if self.vertical_scroll > self.index {
            let height_diff = self.vertical_scroll - self.index;
            self.vertical_scroll -= height_diff;
        }

        self.logs
            .iter()
            .enumerate()
            .skip(self.vertical_scroll)
            .take(height)
            .for_each(|(i, log)| {
                let (label, label_width, label_style) = match log.level {
                    LogLevel::Info => ("Info", 4, Style::new().fg(Color::Green)),
                    LogLevel::Warning => ("Warning", 7, Style::new().fg(Color::Yellow)),
                    LogLevel::Error => ("Error", 5, Style::new().fg(Color::Red)),
                };
                line.push_span(Span::styled(label, label_style));
                line.push_span(Span::raw(" "));

                let (scroll, style) = if self.index == i {
                    let label_width = label_width + 1;
                    let log_width_area = line_area.width.saturating_sub(label_width);
                    let max_scroll = log.width.saturating_sub(log_width_area as usize);
                    self.horizontal_scroll = self.horizontal_scroll.min(max_scroll);
                    (
                        self.horizontal_scroll,
                        Style::new().bg(colors.accent).fg(colors.on_accent).bold(),
                    )
                } else {
                    (0, Style::new())
                };

                line.push_span(Span::styled(&log.message[scroll..], style));

                (&line).render(line_area, buf);
                line.spans.clear();
                line_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, events: &EventHandler) {
        match key {
            KeyCode::Down => {
                let old_index = self.index;
                self.index = usize::min(self.index + 1, self.logs.len().saturating_sub(1));
                if self.index != old_index {
                    self.horizontal_scroll = 0;
                }
                events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                let old_index = self.index;
                self.index = self.index.saturating_sub(1);
                if self.index != old_index {
                    self.horizontal_scroll = 0;
                }
                events.send(AppEvent::Render);
            }
            KeyCode::Right => {
                self.horizontal_scroll += 1;
                events.send(AppEvent::Render);
            }
            KeyCode::Left => {
                self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
                events.send(AppEvent::Render);
            }
            _ => {}
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}

pub struct Log {
    message: String,
    level: LogLevel,
    width: usize,
}

impl Log {
    pub fn new(message: impl Into<String>, level: LogLevel) -> Self {
        let message = message.into();
        let width = unicode_width::UnicodeWidthStr::width(message.as_str());

        Self {
            message,
            level,
            width,
        }
    }
}

pub enum LogLevel {
    Info,
    Warning,
    Error,
}
