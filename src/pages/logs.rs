use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    utils,
    widgets::{List, ListMove},
};

pub struct LogsPage {
    title: String,
    logs: Vec<Log>,
    queue: u32,
    list: List,
    horizontal_scroll: usize,
    events: EventSender,
}

impl LogsPage {
    pub const fn new(events: EventSender) -> Self {
        Self {
            title: String::new(),
            logs: Vec::new(),
            queue: 0,
            list: List::new(),
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

        let mut buffer = itoa::Buffer::new();
        let len = buffer.format(self.logs.len());
        self.title.extend([" Logs (", len, ") "]);

        let block = Block::bordered()
            .title(self.title.as_str())
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1));
        let logs_area = block.inner(area);

        block.render(area, buf);
        self.title.clear();

        self.list.render(
            logs_area,
            buf,
            self.logs.iter(),
            |line, buf, log, is_index, _| {
                let (scroll, style) = if is_index {
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
            },
        );
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) {
        let old_index = self.list.index();

        match key {
            KeyCode::Down => {
                self.list.move_index(ListMove::Down, false);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.list.move_index(ListMove::Up, false);
                self.events.send(AppEvent::Render);
            }
            KeyCode::PageDown => {
                self.list.move_index(ListMove::PageDown, false);
                self.events.send(AppEvent::Render);
            }
            KeyCode::PageUp => {
                self.list.move_index(ListMove::PageUp, false);
                self.events.send(AppEvent::Render);
            }
            KeyCode::End => {
                self.list.move_index(ListMove::End, false);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Home => {
                self.list.move_index(ListMove::Start, false);
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

        if self.list.index() != old_index {
            self.horizontal_scroll = 0;
        }
    }

    pub fn on_exit(&self) {}
}

pub struct Log {
    message: String,
    width: usize,
}

impl Log {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let width = unicode_width::UnicodeWidthStr::width(message.as_str());
        Self { message, width }
    }
}
