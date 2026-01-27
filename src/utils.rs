use std::time::Duration;

use ratatui::layout::Rect;

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
