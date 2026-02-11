use std::{cmp::Ordering, ops::Range};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
    style::{Color, Style},
};
use unicode_segmentation::UnicodeSegmentation;

use super::utils;

pub struct TextInput {
    input: String,
    placeholder: &'static str,
    cursor: usize,
    selector: Option<usize>,
    scroll: usize,
    cursor_style: Style,
    selector_style: Style,
    placeholder_style: Style,
}

pub enum CursorMove {
    Forward,
    Back,
    Up,
    Down,
    Start,
    End,
}

pub enum CursorDelete {
    Forward,
    Back,
}

impl TextInput {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            placeholder: "",
            cursor: 0,
            selector: None,
            scroll: 0,
            cursor_style: Style::new().bg(Color::White).fg(Color::Black),
            selector_style: Style::new().bg(Color::DarkGray).fg(Color::Gray),
            placeholder_style: Style::new().fg(Color::DarkGray).italic(),
        }
    }

    pub const fn with_placeholder(mut self, s: &'static str) -> Self {
        self.placeholder = s;
        self
    }

    pub const fn with_styles(mut self, cursor: Style, selector: Style, placeholder: Style) -> Self {
        self.cursor_style = cursor;
        self.selector_style = selector;
        self.placeholder_style = placeholder;
        self
    }

    pub const fn as_str(&self) -> &str {
        self.input.as_str()
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Right => self.move_cursor(CursorMove::Forward, shift),
            KeyCode::Left => self.move_cursor(CursorMove::Back, shift),
            KeyCode::Up => self.move_cursor(CursorMove::Up, shift),
            KeyCode::Down => self.move_cursor(CursorMove::Down, shift),
            KeyCode::Home => self.move_cursor(CursorMove::Start, shift),
            KeyCode::End => self.move_cursor(CursorMove::End, shift),
            KeyCode::Backspace => self.delete(CursorDelete::Back),
            KeyCode::Delete => self.delete(CursorDelete::Forward),
            KeyCode::Char(c) => match c {
                'a' => {
                    if ctrl {
                        self.select_all()
                    } else {
                        self.push_char(c);
                        true
                    }
                }
                _ => {
                    self.push_char(c);
                    true
                }
            },
            _ => false,
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.delete_selection();

        let c = if c.is_whitespace() { ' ' } else { c };
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn _push_str(&mut self, s: &str) {
        self.delete_selection();

        s.graphemes(true)
            .map(|g| {
                if g.chars().any(|c| c.is_whitespace()) {
                    " "
                } else {
                    g
                }
            })
            .for_each(|g| {
                self.input.insert_str(self.cursor, g);
                self.cursor += g.len();
            });
    }

    pub fn move_cursor(&mut self, cm: CursorMove, shift: bool) -> bool {
        let (old_cursor, old_selector) = (self.cursor, self.selector);

        if shift {
            if self.selector.is_none() {
                self.selector = Some(self.cursor);
            }
        } else {
            self.selector = None;
        }

        match cm {
            CursorMove::Forward => {
                if let Some(g) = self.input[self.cursor..].graphemes(true).next() {
                    self.cursor += g.len();
                }
            }
            CursorMove::Back => {
                if let Some(g) = self.input[..self.cursor].graphemes(true).rev().next() {
                    self.cursor -= g.len();
                }
            }
            CursorMove::Up | CursorMove::Start => {
                self.cursor = 0;
            }
            CursorMove::Down | CursorMove::End => {
                self.cursor = self.input.len();
            }
        }

        self.selector.take_if(|s| *s == self.cursor);

        self.cursor != old_cursor || self.selector != old_selector
    }

    pub fn select_all(&mut self) -> bool {
        let (old_cursor, old_selector) = (self.cursor, self.selector);

        self.cursor = self.input.len();
        self.selector = Some(0);

        self.cursor != old_cursor || self.selector != old_selector
    }

    pub fn delete(&mut self, cd: CursorDelete) -> bool {
        if self.delete_selection() {
            return true;
        }

        match cd {
            CursorDelete::Forward => match self.input[self.cursor..].graphemes(true).next() {
                Some(g) => {
                    self.input
                        .replace_range(self.cursor..self.cursor + g.len(), "");
                    true
                }
                None => false,
            },
            CursorDelete::Back => match self.input[..self.cursor].graphemes(true).rev().next() {
                Some(g) => {
                    self.cursor -= g.len();
                    self.input
                        .replace_range(self.cursor..self.cursor + g.len(), "");
                    true
                }
                None => false,
            },
        }
    }

    pub fn _clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.selector = None;
        self.scroll = 0;
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if self.input.is_empty() {
            let Rect { x, y, .. } = area;
            buf.set_string(x, y, self.placeholder, self.placeholder_style);
            buf[(x, y)].set_style(self.cursor_style);
            return;
        }

        // Get total input width and update scroll
        let total_width = unicode_width::UnicodeWidthStr::width(self.input.as_str());
        self.scroll =
            utils::calculate_scroll(total_width, area.width, self.cursor, self.scroll, 0, 0, 0);

        // Render
        let (selection_start, selection_end) = {
            let selection = self.try_selection().unwrap_or(self.cursor..self.cursor);
            (selection.start, selection.end)
        };

        let max_width = area.width as usize;
        let mut current_width = 0;
        let Rect { mut x, y, .. } = area;

        for (i, g) in self.input.grapheme_indices(true) {
            let grapheme_width = unicode_width::UnicodeWidthStr::width(g);
            current_width += grapheme_width;

            if current_width > max_width + self.scroll {
                break;
            } else if current_width > self.scroll {
                let is_cursor = i == self.cursor;
                let is_selected = i >= selection_start && i < selection_end;
                let style = if is_cursor {
                    self.cursor_style
                } else if is_selected {
                    self.selector_style
                } else {
                    Style::new()
                };
                (x, _) = buf.set_stringn(x, y, g, grapheme_width, style);
            }
        }

        if self.cursor == self.input.len() {
            buf[(x, y)].set_style(self.cursor_style);
        }
    }

    fn selection(&self, selector: usize) -> Option<Range<usize>> {
        match self.cursor.cmp(&selector) {
            Ordering::Less => Some(self.cursor..selector),
            Ordering::Greater => Some(selector..self.cursor),
            Ordering::Equal => None,
        }
    }

    fn try_selection(&self) -> Option<Range<usize>> {
        self.selector.and_then(|selector| self.selection(selector))
    }

    fn take_selection(&mut self) -> Option<Range<usize>> {
        self.selector
            .take()
            .and_then(|selector| self.selection(selector))
    }

    fn delete_selection(&mut self) -> bool {
        let Some(range) = self.take_selection() else {
            return false;
        };
        self.cursor = range.start;
        self.input.replace_range(range, "");
        true
    }
}
