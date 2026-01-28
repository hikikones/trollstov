use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct TextSegment {
    text: String,
    segments: Vec<(usize, Style)>,
    total_width: usize,
    alignment: Alignment,
}

impl TextSegment {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            segments: Vec::new(),
            total_width: 0,
            alignment: Alignment::Left,
        }
    }

    pub const fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    pub const fn width(&self) -> u16 {
        self.total_width as u16
    }

    pub fn push_char(&mut self, ch: char, style: Style) {
        self.text.push(ch);
        self.segments.push((ch.len_utf8(), style));
        self.total_width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
    }

    pub fn push_chars(&mut self, chars: &[char], style: Style) {
        let mut len = 0;
        let mut width = 0;

        for ch in chars.iter().copied() {
            len += ch.len_utf8();
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            self.text.push(ch);
        }

        self.segments.push((len, style));
        self.total_width += width;
    }

    pub fn push_str(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }

        self.text.push_str(text);
        self.segments.push((text.len(), style));
        self.total_width += unicode_width::UnicodeWidthStr::width(text);
    }

    pub fn extend<'a>(&mut self, items: impl IntoIterator<Item = (&'a str, Style)>) {
        for (text, style) in items.into_iter() {
            self.push_str(text, style);
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.segments.clear();
        self.total_width = 0;
    }

    pub fn render(&self, line: Rect, buf: &mut Buffer) {
        let line = match self.alignment {
            Alignment::Left => line,
            Alignment::Center => Rect {
                x: line.x + (line.width.saturating_sub(self.width())) / 2,
                ..line
            },
            Alignment::Right => Rect {
                x: line.x + line.width.saturating_sub(self.width()),
                ..line
            },
        };
        let mut start = 0;
        let Rect {
            mut x,
            y,
            mut width,
            ..
        } = line;

        for (len, style) in self.segments.iter().copied() {
            let end = start + len;
            let text = &self.text[start..end];

            let (next_x, _) = buf.set_stringn(x, y, text, width as usize, style);
            width -= next_x - x;
            x = next_x;
            start = end;

            if width == 0 {
                break;
            }
        }
    }
}

pub struct Shortcut<'a> {
    name: &'a str,
    key: &'a str,
}

impl<'a> Shortcut<'a> {
    pub const fn new(name: &'a str, key: &'a str) -> Self {
        Self { name, key }
    }
}

pub struct Shortcuts<'a> {
    name_color: Color,
    key_color: Color,
    line: Line<'a>,
}

impl<'a> Shortcuts<'a> {
    pub fn new(name_color: Color, key_color: Color) -> Self {
        Self {
            name_color,
            key_color,
            line: Line::default().centered(),
        }
    }

    pub fn push(&mut self, shortcut: Shortcut<'a>) {
        let spans = [
            Span::raw(" "),
            Span::styled(shortcut.key, self.key_color),
            Span::raw(" "),
            Span::styled(shortcut.name, self.name_color),
            Span::raw(" "),
        ];
        self.line.spans.extend(spans);
    }

    pub fn extend(&mut self, shortcuts: impl IntoIterator<Item = Shortcut<'a>>) {
        for shortcut in shortcuts {
            self.push(shortcut);
        }
    }

    pub fn clear(&mut self) {
        self.line.spans.clear();
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        (&self.line).render(area, buf);
    }
}

pub struct TextInput {
    input: String,
    placeholder: &'static str,
    cursor_fg: Color,
    cursor_bg: Color,
    placeholder_fg: Color,
    cursor_index: usize,
    cursor_column: usize,
    selection_start: Option<usize>,
    scroll: usize,
    spans: Vec<Span<'static>>,
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
    _Selection,
}

impl TextInput {
    pub const fn new(cursor_fg: Color, cursor_bg: Color, placeholder_fg: Color) -> Self {
        Self {
            input: String::new(),
            placeholder: "",
            cursor_fg,
            cursor_bg,
            placeholder_fg,
            cursor_index: 0,
            cursor_column: 0,
            selection_start: None,
            scroll: 0,
            spans: Vec::new(),
        }
    }

    pub const fn with_placeholder(mut self, s: &'static str) -> Self {
        self.placeholder = s;
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    pub const fn as_str(&self) -> &str {
        self.input.as_str()
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Right => return self.move_cursor(CursorMove::Forward, shift),
            KeyCode::Left => return self.move_cursor(CursorMove::Back, shift),
            KeyCode::Up => return self.move_cursor(CursorMove::Up, shift),
            KeyCode::Down => return self.move_cursor(CursorMove::Down, shift),
            KeyCode::Home => return self.move_cursor(CursorMove::Start, shift),
            KeyCode::End => return self.move_cursor(CursorMove::End, shift),
            KeyCode::Backspace => return self.delete(CursorDelete::Back),
            KeyCode::Delete => return self.delete(CursorDelete::Forward),
            KeyCode::Char(c) => match c {
                'a' => {
                    if ctrl {
                        return self.select_all();
                    }

                    self.push_char(c);
                    return true;
                }
                'c' => {
                    if ctrl {
                        if let Some(selector) = self.selection_start {
                            if let Some(range) = self.get_selection_range(selector) {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(&self.input[range]);
                                }
                            }
                        }
                    } else {
                        self.push_char(c);
                        return true;
                    }
                }
                'v' => {
                    if ctrl {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(s) = clipboard.get_text() {
                                self.push_str(&s);
                                return true;
                            }
                        }
                    } else {
                        self.push_char(c);
                        return true;
                    }
                }
                _ => {
                    self.push_char(c);
                    return true;
                }
            },
            _ => {}
        }

        false
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(start) = self.selection_start.take() {
            self.delete_selection(start);
        }
        let c = if c.is_whitespace() { ' ' } else { c };
        self.input.insert(self.cursor_index, c);
        self.cursor_index += c.len_utf8();
    }

    pub fn push_str(&mut self, s: &str) {
        if let Some(start) = self.selection_start.take() {
            self.delete_selection(start);
        }
        s.graphemes(true)
            .map(|g| {
                if g.chars().any(|c| c.is_whitespace()) {
                    " "
                } else {
                    g
                }
            })
            .for_each(|g| {
                self.input.insert_str(self.cursor_index, g);
                self.cursor_index += g.len();
            });
    }

    pub fn move_cursor(&mut self, cm: CursorMove, shift: bool) -> bool {
        let (old_cursor, old_selector) = (self.cursor_index, self.selection_start);

        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_index);
            }
        } else {
            self.selection_start = None;
        }

        match cm {
            CursorMove::Forward => {
                if let Some(g) = self.input[self.cursor_index..].graphemes(true).next() {
                    self.cursor_index += g.len();
                }
            }
            CursorMove::Back => {
                if let Some(g) = self.input[..self.cursor_index].graphemes(true).rev().next() {
                    self.cursor_index -= g.len();
                }
            }
            CursorMove::Up | CursorMove::Start => {
                self.cursor_index = 0;
            }
            CursorMove::Down | CursorMove::End => {
                self.cursor_index = self.input.len();
            }
        }

        self.selection_start.take_if(|s| *s == self.cursor_index);

        self.cursor_index != old_cursor || self.selection_start != old_selector
    }

    pub fn select_all(&mut self) -> bool {
        let (old_cursor, old_selector) = (self.cursor_index, self.selection_start);

        self.cursor_index = self.input.len();
        self.selection_start = Some(0);

        self.cursor_index != old_cursor || self.selection_start != old_selector
    }

    pub fn delete(&mut self, cd: CursorDelete) -> bool {
        match cd {
            CursorDelete::Forward => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => match self.input[self.cursor_index..].graphemes(true).next() {
                    Some(g) => {
                        self.input
                            .replace_range(self.cursor_index..self.cursor_index + g.len(), "");
                        true
                    }
                    None => false,
                },
            },
            CursorDelete::Back => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => match self.input[..self.cursor_index].graphemes(true).rev().next() {
                    Some(g) => {
                        self.cursor_index -= g.len();
                        self.input
                            .replace_range(self.cursor_index..self.cursor_index + g.len(), "");
                        true
                    }
                    None => false,
                },
            },
            CursorDelete::_Selection => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => false,
            },
        }
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_index = 0;
        self.cursor_column = 0;
        self.selection_start = None;
        self.scroll = 0;
        self.spans.clear();
    }

    fn get_selection_range(&self, selector: usize) -> Option<std::ops::Range<usize>> {
        match self.cursor_index.cmp(&selector) {
            std::cmp::Ordering::Less => Some(self.cursor_index..selector),
            std::cmp::Ordering::Greater => Some(selector..self.cursor_index),
            std::cmp::Ordering::Equal => None,
        }
    }

    fn delete_selection(&mut self, selector: usize) -> bool {
        let Some(range) = self.get_selection_range(selector) else {
            return false;
        };
        self.cursor_index = range.start;
        self.input.replace_range(range, "");
        true
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.spans.clear();
        self.cursor_column = 0;

        let mut total_width = 0;
        let input_len = self.input.len();
        let selection_start = self
            .cursor_index
            .min(self.selection_start.unwrap_or(self.cursor_index));
        let selection_end = self
            .selection_start
            .unwrap_or(self.cursor_index)
            .max(self.cursor_index);
        let cursor_style = Style::new().bg(self.cursor_bg).fg(self.cursor_fg);
        let selector_style = cursor_style;

        let mut graphemes = self.input.grapheme_indices(true);

        loop {
            let Some((i, g)) = graphemes.next() else {
                if self.cursor_index == input_len {
                    self.cursor_column = total_width;
                    self.spans.push(Span::styled(" ", cursor_style));
                }
                break;
            };

            let is_cursor = i == self.cursor_index;
            let is_selected = i >= selection_start && i < selection_end;

            let style = if is_cursor {
                self.cursor_column = total_width;
                cursor_style
            } else if is_selected {
                selector_style
            } else {
                Style::new()
            };

            let span = Span::styled(g.to_string(), style);
            total_width += span.width();
            self.spans.push(span);
        }

        if self.input.is_empty() {
            self.spans.push(Span::styled(
                self.placeholder,
                Style::new().italic().fg(self.placeholder_fg),
            ));
        }

        // todo: fix scroll when left-most char has width > 1
        let line_width = area.width as usize;
        if self.cursor_column > self.scroll {
            let width_diff = self.cursor_column - self.scroll;
            let line_width = line_width.saturating_sub(1);
            if width_diff > line_width {
                self.scroll += width_diff - line_width;
            }
        } else if self.scroll > self.cursor_column {
            let width_diff = self.scroll - self.cursor_column;
            self.scroll -= width_diff;
        }

        let mut skip_width = 0;
        let mut input_width = 0;
        let mut span_area = Rect { height: 1, ..area };

        for span in self.spans.iter() {
            let span_width = span.width();
            skip_width += span_width;
            if skip_width > self.scroll && input_width < line_width {
                input_width += span_width;
                span_area.width = span_width as u16;
                (&span).render(span_area, buf);
                span_area.x += span_width as u16;
            }
        }
    }
}
