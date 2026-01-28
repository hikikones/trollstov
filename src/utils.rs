use std::time::Duration;

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

/// Formats the duration as `mm:ss` to a String.
pub fn format_duration(duration: Duration) -> String {
    let mut s = String::with_capacity(5);
    format_duration_in_place(duration, &mut s);
    s
}

/// Formats the duration as `mm:ss` and appends it to the mutable String.
pub fn format_duration_in_place(duration: Duration, s: &mut String) {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();

    if minutes < 10 {
        s.push('0');
        s.push_str(buffer.format(minutes));
    } else if minutes < 100 {
        s.push_str(buffer.format(minutes));
    } else {
        s.push_str("99:99");
        return;
    }

    s.push(':');

    if seconds < 10 {
        s.push('0');
        s.push_str(buffer.format(seconds));
    } else {
        s.push_str(buffer.format(seconds));
    }
}

/// Formats the duration as `mm:ss` and returns a stack-allocated char array.
pub fn format_duration_on_stack(duration: Duration) -> [char; 5] {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();
    let mut chars = ['0', '0', ':', '0', '0'];

    if minutes < 10 {
        chars[1] = unsafe { buffer.format(minutes).chars().next().unwrap_unchecked() };
    } else if minutes < 100 {
        for (i, char) in buffer.format(minutes).chars().enumerate() {
            chars[i] = char;
        }
    } else {
        return ['9', '9', ':', '9', '9'];
    }

    if seconds < 10 {
        chars[4] = unsafe { buffer.format(seconds).chars().next().unwrap_unchecked() };
    } else {
        for (i, char) in buffer.format(seconds).chars().enumerate() {
            chars[i + 3] = char;
        }
    }

    chars
}

/// Calculates the amount of lines to scroll/skip
/// given the index (current line) and height of area.
pub fn calculate_scroll(index: usize, height: u16, mut scroll: usize) -> usize {
    let height = height as usize;
    if index > scroll {
        let height_diff = index - scroll;
        let height = height.saturating_sub(1);
        if height_diff > height {
            scroll += height_diff - height;
        }
    } else if scroll > index {
        let height_diff = scroll - index;
        scroll -= height_diff;
    }
    scroll
}
