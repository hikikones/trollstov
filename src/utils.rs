use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::Widget,
};

/// Aligns the inner [Rect] inside the outer [Rect].
/// Assumes the inner rect fits inside outer rect.
pub fn align(inner: Rect, outer: Rect, alignment: Alignment) -> Rect {
    match alignment {
        Alignment::TopLeft => todo!(),
        Alignment::Top => todo!(),
        Alignment::TopCenter => todo!(),
        Alignment::TopRight => todo!(),
        Alignment::Right => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            ..inner
        },
        Alignment::RightCenter => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
            ..inner
        },
        Alignment::BottomRight => todo!(),
        Alignment::Bottom => todo!(),
        Alignment::BottomCenter => todo!(),
        Alignment::BottomLeft => todo!(),
        Alignment::Left => todo!(),
        Alignment::LeftCenter => todo!(),
        Alignment::Center => Rect {
            x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
            y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
            ..inner
        },
        Alignment::CenterHorizontal => Rect {
            x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
            ..inner
        },
        Alignment::CenterVertical => Rect {
            y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
            ..inner
        },
    }
}

pub enum Alignment {
    TopLeft,
    Top,
    TopCenter,
    TopRight,
    Right,
    RightCenter,
    BottomRight,
    Bottom,
    BottomCenter,
    BottomLeft,
    Left,
    LeftCenter,
    Center,
    CenterHorizontal,
    CenterVertical,
}

/// Formats the duration as `mm:ss` and appends it to the mutable String.
pub fn format_duration(duration: Duration, s: &mut String) {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();

    if minutes < 10 {
        s.push('0');
        s.push_str(buffer.format(minutes));
    } else {
        s.push_str(buffer.format(minutes));
    }

    s.push(':');

    if seconds < 10 {
        s.push('0');
        s.push_str(buffer.format(seconds));
    } else {
        s.push_str(buffer.format(seconds));
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
