use ratatui::{buffer::Buffer, layout::Rect, style::Style};

/// Prints `ascii` assuming only ASCII, no newlines or
/// control characters for simple layout calculation.
pub fn print_ascii(
    area: Rect,
    buf: &mut Buffer,
    ascii: impl AsRef<str>,
    style: Style,
    alignment: Alignment,
) {
    let ascii = ascii.as_ref();

    let Rect {
        mut x,
        y,
        mut width,
        ..
    } = align(
        Rect {
            width: ascii.len() as u16,
            height: 1,
            ..area
        },
        area,
        alignment,
    );

    for ch in ascii.chars() {
        if width == 0 {
            break;
        }

        buf[(x, y)].set_char(ch).set_style(style);

        x += 1;
        width -= 1;
    }
}

/// Prints `text` and fills remaining empty cells with the given style.
pub fn print_line(line: Rect, buf: &mut Buffer, text: impl AsRef<str>, style: Style) {
    let Rect { x, y, width, .. } = line;
    let (end_x, _) = buf.set_stringn(x, y, text, width as usize, style);
    let remaining = width - (end_x - x);
    for i in 0..remaining {
        buf[(end_x + i, y)].set_style(style);
    }
}

/// Prints a collection of text slices and fills remaining empty cells with the given style.
pub fn print_line_iter(
    line: Rect,
    buf: &mut Buffer,
    texts: impl IntoIterator<Item = impl AsRef<str>>,
    style: Style,
) {
    let Rect {
        mut x,
        y,
        mut width,
        ..
    } = line;

    for text in texts {
        let (next_x, _) = buf.set_stringn(x, y, text, width as usize, style);
        width -= next_x - x;
        x = next_x;
        if width == 0 {
            break;
        }
    }

    for i in 0..width {
        buf[(x + i, y)].set_style(style);
    }
}

/// Prints a collection of text segments with widths and spacing.
/// Fills remaining empty cells with the given style.
pub fn print_text_segments(
    line: Rect,
    buf: &mut Buffer,
    segments: impl IntoIterator<Item = (impl AsRef<str>, u16, u16)>,
    style: Style,
) {
    let Rect { mut x, y, .. } = line;
    for (text, width, spacing) in segments {
        let text_width = width.saturating_sub(spacing);
        let (next_x, _) = buf.set_stringn(x, y, text, text_width as usize, style);
        let remaining = width - (next_x - x);
        for i in 0..remaining {
            buf[(next_x + i, y)].set_style(style);
        }
        x = next_x + remaining;
    }
}

/// Aligns the inner [Rect] inside the outer [Rect].
/// Assumes the inner rect fits inside outer rect.
pub fn align(inner: Rect, outer: Rect, alignment: Alignment) -> Rect {
    match alignment {
        Alignment::TopLeft => Rect {
            x: outer.x,
            y: outer.y,
            ..inner
        },
        Alignment::Top => Rect {
            y: outer.y,
            ..inner
        },
        Alignment::TopCenter => Rect {
            x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
            y: outer.y,
            ..inner
        },
        Alignment::TopRight => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            y: outer.y,
            ..inner
        },
        Alignment::Right => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            ..inner
        },
        Alignment::RightCenter => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
            ..inner
        },
        Alignment::BottomRight => Rect {
            x: outer.x + outer.width.saturating_sub(inner.width),
            y: outer.y + outer.height.saturating_sub(inner.height),
            ..inner
        },
        Alignment::Bottom => Rect {
            y: outer.y + outer.height.saturating_sub(inner.height),
            ..inner
        },
        Alignment::BottomCenter => Rect {
            x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
            y: outer.y + outer.height.saturating_sub(inner.height),
            ..inner
        },
        Alignment::BottomLeft => Rect {
            x: outer.x,
            y: outer.y + outer.height.saturating_sub(inner.height),
            ..inner
        },
        Alignment::Left => Rect {
            x: outer.x,
            ..inner
        },
        Alignment::LeftCenter => Rect {
            x: outer.x,
            y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
            ..inner
        },
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

#[allow(unused)]
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
