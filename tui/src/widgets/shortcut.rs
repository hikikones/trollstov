use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::Widget,
};

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
