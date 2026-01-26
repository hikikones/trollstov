use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct TextSegment {
    text: String,
    styles: Vec<(Style, usize, usize)>,
    total_width: usize,
}

impl TextSegment {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            styles: Vec::new(),
            total_width: 0,
        }
    }

    pub const fn width(&self) -> u16 {
        self.total_width as u16
    }

    pub fn push(&mut self, text: impl AsRef<str>, style: Style) {
        let text = text.as_ref();

        if text.is_empty() {
            return;
        }

        let len = text.len();
        let width = unicode_width::UnicodeWidthStr::width(text);

        self.text.push_str(text);
        self.styles.push((style, len, width));
        self.total_width += width;
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = (impl AsRef<str>, Style)>) {
        for (text, style) in items.into_iter() {
            self.push(text, style);
        }
    }

    pub fn pop(&mut self) {
        if let Some((_, len, width)) = self.styles.pop() {
            self.text.truncate(self.text.len() - len);
            self.total_width -= width;
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.styles.clear();
        self.total_width = 0;
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let max_width = area.width as usize;
        let mut start = 0;
        let mut current_width = 0;
        let Rect { mut x, mut y, .. } = area;

        for (style, len, width) in self.styles.iter().copied() {
            let end = start + len;
            let text = &self.text[start..end];

            current_width += width;
            if current_width > max_width {
                let remaining = max_width - (current_width - width);
                if remaining > 0 {
                    buf.set_stringn(x, y, text, remaining, style);
                }
                break;
            } else {
                (x, y) = buf.set_stringn(x, y, text, width, style);
            }

            start = end;
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
