use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
};

use crate::widgets::TextSegment;

pub struct Shortcut<'a> {
    name: &'a str,
    key: &'a str,
}

impl<'a> Shortcut<'a> {
    pub const fn new(name: &'a str, key: &'a str) -> Self {
        Self { name, key }
    }
}

pub struct Shortcuts {
    name_color: Color,
    key_color: Color,
    text: TextSegment,
}

impl Shortcuts {
    pub fn new(name_color: Color, key_color: Color) -> Self {
        Self {
            name_color,
            key_color,
            text: TextSegment::new().with_alignment(Alignment::Center),
        }
    }

    pub fn push(&mut self, shortcut: Shortcut<'_>) {
        if !self.text.is_empty() {
            self.text.push_char(' ', Style::new());
        }

        self.text.extend([
            (shortcut.key, Style::new().fg(self.key_color)),
            (" ", Style::new()),
            (shortcut.name, Style::new().fg(self.name_color)),
        ]);
    }

    pub fn push_iter<'a>(&mut self, name: impl IntoIterator<Item = &'a str>, key: &str) {
        if !self.text.is_empty() {
            self.text.push_char(' ', Style::new());
        }

        self.text
            .extend([(key, Style::new().fg(self.key_color)), (" ", Style::new())]);

        self.text.extend_as_one(name, self.name_color);
    }

    pub fn extend<'a>(&mut self, shortcuts: impl IntoIterator<Item = Shortcut<'a>>) {
        for shortcut in shortcuts {
            self.push(shortcut);
        }
    }

    pub fn pop(&mut self) {
        if self.text.is_empty() {
            return;
        }

        self.text.pop();
        self.text.pop();
        self.text.pop();
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        self.text.render(area, buf);
    }
}
