use std::time::Duration;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

/// Prints the str `s` assuming only ASCII, no newlines or
/// control characters for easy layout calculation.
pub fn print_ascii(
    area: Rect,
    buf: &mut Buffer,
    s: impl AsRef<str>,
    style: Style,
    alignment: ratatui::layout::Alignment,
) {
    let s = s.as_ref();
    let x = match alignment {
        ratatui::layout::Alignment::Left => area.x,
        ratatui::layout::Alignment::Center => {
            area.x + (area.width.saturating_sub(s.len() as u16)) / 2
        }
        ratatui::layout::Alignment::Right => area.x + area.width.saturating_sub(s.len() as u16),
    };
    buf.set_stringn(x, area.y, s, s.len(), style);
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

/// Calculates the amount of lines to scroll given the height and current index.
pub fn calculate_scroll(height: u16, index: usize, mut scroll: usize) -> usize {
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
