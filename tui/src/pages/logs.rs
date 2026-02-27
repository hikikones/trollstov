use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    colors::Colors,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, utils},
};

pub struct LogsPage {
    title: String,
    logs: Vec<Log>,
    queue: u32,
    list: List,
    horizontal_scroll: usize,
}

impl LogsPage {
    pub const fn new(colors: &Colors) -> Self {
        Self {
            title: String::new(),
            logs: Vec::new(),
            queue: 0,
            list: List::new().with_colors(colors.neutral, None),
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
            utils::print_ascii(
                area,
                buf,
                "No logs to report",
                colors.neutral,
                utils::Alignment::Center,
            );
            return;
        }

        jukebox::utils::format_int(self.logs.len(), |len| {
            self.title.extend([" Logs (", len, ") "]);
        });

        let block = Block::bordered()
            .title(self.title.as_str())
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1));
        let logs_area = block.inner(area);

        block.render(area, buf);
        self.title.clear();

        self.list
            .render(logs_area, buf, self.logs.iter(), |line, buf, log, item| {
                let (scroll, style) = if item == ListItem::Selected {
                    let max_scroll = log.width.saturating_sub(line.width as usize);
                    self.horizontal_scroll = max_scroll.min(self.horizontal_scroll);
                    (
                        self.horizontal_scroll,
                        Style::new().bg(colors.accent).fg(colors.on_accent),
                    )
                } else {
                    (0, Style::new())
                };

                utils::print_line(line, buf, &log.message[scroll..], style);
            });

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
                        self.list.move_index(ListMove::Custom(0), false);
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
