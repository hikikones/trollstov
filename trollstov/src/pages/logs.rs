use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use widgets::{List, ListItem, Shortcut, Shortcuts};

use crate::{app::Action, settings::Colors};

// TODO: LogLevel? ERROR/INFO.

pub struct LogsPage {
    logs: Vec<Log>,
    queue: u32,
    list: List,
    horizontal_scroll: usize,
}

impl LogsPage {
    pub const fn new() -> Self {
        Self {
            logs: Vec::new(),
            queue: 0,
            list: List::new(),
            horizontal_scroll: 0,
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

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        if self.logs.is_empty() {
            widgets::print_ascii(
                area,
                buf,
                "No logs to report",
                colors.neutral,
                Some(widgets::Alignment::Center),
            );
            return;
        }

        // Bordered block for logs
        let block = Block::bordered().padding(Padding::horizontal(1));
        let logs_area = block.inner(area);
        block.render(area, buf);

        // Title for bordered logs
        utils::format_int(self.logs.len(), |len| {
            widgets::print_asciis(
                Rect {
                    y: area.y,
                    height: 1,
                    ..logs_area
                },
                buf,
                [" Logs (", len, ") "],
                Style::new(),
                Some(widgets::Alignment::CenterHorizontal),
            );
        });

        // Render logs
        self.list.set_colors(colors.neutral, None).render(
            logs_area,
            buf,
            self.logs.iter(),
            |line, buf, log, item| {
                let (scroll, style) = if item == ListItem::Selected {
                    let max_scroll = log.width.saturating_sub(line.width as usize);
                    self.horizontal_scroll = max_scroll.min(self.horizontal_scroll);
                    (
                        self.horizontal_scroll,
                        Style::new().bg(colors.primary).fg(colors.on_primary),
                    )
                } else {
                    (0, Style::new())
                };

                widgets::print_text(line, buf, &log.message[scroll..], style, true, None);
            },
        );

        // Shortcuts
        shortcuts.push(Shortcut::new("Clear", "c"));
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Action {
        match key {
            KeyCode::Right => {
                self.horizontal_scroll += 1;
                return Action::Render;
            }
            KeyCode::Left => {
                self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
                return Action::Render;
            }
            KeyCode::Char(c) => match c {
                'c' => {
                    if !self.logs.is_empty() {
                        self.logs.clear();
                        self.horizontal_scroll = 0;
                        self.list.set_index(0);
                        return Action::Render;
                    }
                }
                _ => {}
            },
            _ => {
                if self.list.input(key, KeyModifiers::empty()) {
                    self.horizontal_scroll = 0;
                    return Action::Render;
                }
            }
        }

        Action::None
    }

    pub fn on_exit(&self) {}
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
