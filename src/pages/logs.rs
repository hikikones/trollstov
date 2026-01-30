use std::path::PathBuf;

use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    utils,
};

pub struct LogsPage {
    logs: Vec<Log>,
    queue: u32,
    index: usize,
    vertical_scroll: usize,
    horizontal_scroll: usize,
    events: EventSender,
}

impl LogsPage {
    pub const fn new(events: EventSender) -> Self {
        Self {
            logs: Vec::new(),
            queue: 0,
            index: 0,
            vertical_scroll: 0,
            horizontal_scroll: 0,
            events,
        }
    }

    pub fn enqueue(&mut self, log: Log) {
        self.logs.push(log);
        self.queue += 1;
    }

    pub const fn queue_len(&self) -> u32 {
        self.queue
    }

    pub fn on_enter(&mut self) {
        self.queue = 0;
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, colors: &Colors) {
        if self.logs.is_empty() {
            utils::print_ascii(
                area,
                buf,
                "No logs to report",
                Style::new().fg(colors.neutral),
                utils::Alignment::Center,
            );
            return;
        }

        self.vertical_scroll =
            utils::calculate_scroll(self.index, area.height, self.vertical_scroll);
        self.render_logs(area, buf, colors);
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) {
        match key {
            KeyCode::Down => {
                let old_index = self.index;
                self.index = usize::min(self.index + 1, self.logs.len().saturating_sub(1));
                if self.index != old_index {
                    self.horizontal_scroll = 0;
                }
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                let old_index = self.index;
                self.index = self.index.saturating_sub(1);
                if self.index != old_index {
                    self.horizontal_scroll = 0;
                }
                self.events.send(AppEvent::Render);
            }
            KeyCode::Right => {
                self.horizontal_scroll += 1;
                self.events.send(AppEvent::Render);
            }
            KeyCode::Left => {
                self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            _ => {}
        }
    }

    pub fn on_exit(&self) {}

    fn render_logs(&mut self, area: Rect, buf: &mut Buffer, colors: &Colors) {
        let mut line = Rect { height: 1, ..area };

        self.logs
            .iter()
            .enumerate()
            .skip(self.vertical_scroll)
            .take(area.height as usize)
            .for_each(|(i, log)| {
                let (scroll, style) = if self.index == i {
                    let max_scroll = log.width.saturating_sub(line.width as usize);
                    self.horizontal_scroll = max_scroll.min(self.horizontal_scroll);
                    (
                        self.horizontal_scroll,
                        Style::new().bg(colors.accent).fg(colors.on_accent).bold(),
                    )
                } else {
                    (0, Style::new())
                };

                utils::print_line(line, buf, &log.message[scroll..], style);

                line.y += 1;
            });
    }
}

pub struct Log {
    message: String,
    width: usize,
}

impl Log {
    pub fn new(message: impl ToString) -> Self {
        let message = message.to_string();
        let width = unicode_width::UnicodeWidthStr::width(message.as_str());
        Self { message, width }
    }
}

impl<E> From<E> for Log
where
    E: std::error::Error,
{
    fn from(value: E) -> Self {
        Log::new(value)
    }
}
