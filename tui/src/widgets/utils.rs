use ratatui::{buffer::Buffer, layout::Rect, style::Style};

/// Prints ascii assuming only ASCII, no newlines or control characters.
pub fn print_ascii(
    line: Rect,
    buf: &mut Buffer,
    ascii: impl AsRef<str>,
    style: impl Into<Style>,
    alignment: Option<Alignment>,
) -> Rect {
    let ascii = ascii.as_ref();
    debug_assert!(ascii.is_ascii());

    let (mut x, y, mut width) = match alignment {
        Some(alignment) => {
            let Rect { x, y, .. } = align(
                Rect {
                    width: line.width.min(ascii.len() as u16),
                    height: 1,
                    ..line
                },
                line,
                alignment,
            );
            let width = (line.x + line.width).saturating_sub(x);
            (x, y, width)
        }
        None => (line.x, line.y, line.width),
    };

    let style = style.into();
    for ch in ascii.chars() {
        if width == 0 {
            break;
        }

        let Some(cell) = buf.cell_mut((x, y)) else {
            break;
        };

        cell.set_char(ch).set_style(style);

        x += 1;
        width -= 1;
    }

    Rect {
        x,
        y,
        width,
        ..line
    }
}

/// Prints ascii collection assuming only ASCII, no newlines or control characters.
pub fn print_asciis<'a>(
    line: Rect,
    buf: &mut Buffer,
    asciis: impl IntoIterator<IntoIter: Clone, Item = &'a str>,
    style: impl Into<Style>,
    alignment: Option<Alignment>,
) -> Rect {
    let asciis = asciis.into_iter();

    let (mut x, y, mut width) = match alignment {
        Some(alignment) => {
            let ascii_width = asciis.clone().map(|s| s.len() as u16).sum::<u16>();
            let Rect { x, y, .. } = align(
                Rect {
                    width: line.width.min(ascii_width),
                    ..line
                },
                line,
                alignment,
            );
            let width = (line.x + line.width).saturating_sub(x);
            (x, y, width)
        }
        None => (line.x, line.y, line.width),
    };

    let style = style.into();
    'outer: for ascii in asciis {
        debug_assert!(ascii.is_ascii());
        for ch in ascii.chars() {
            if width == 0 {
                break 'outer;
            }

            let Some(cell) = buf.cell_mut((x, y)) else {
                break 'outer;
            };

            cell.set_char(ch).set_style(style);

            x += 1;
            width -= 1;
        }
    }

    Rect {
        x,
        y,
        width,
        ..line
    }
}

/// Prints ascii collection with styles assuming only ASCII, no newlines or control characters.
pub fn print_asciis_with_styles<'a>(
    line: Rect,
    buf: &mut Buffer,
    asciis: impl IntoIterator<IntoIter: Clone, Item = (&'a str, Style)>,
    alignment: Option<Alignment>,
) -> Rect {
    let asciis = asciis.into_iter();

    let (mut x, y, mut width) = match alignment {
        Some(alignment) => {
            let ascii_width = asciis.clone().map(|(s, _)| s.len() as u16).sum::<u16>();
            let Rect { x, y, .. } = align(
                Rect {
                    width: line.width.min(ascii_width),
                    ..line
                },
                line,
                alignment,
            );
            let width = (line.x + line.width).saturating_sub(x);
            (x, y, width)
        }
        None => (line.x, line.y, line.width),
    };

    'outer: for (ascii, style) in asciis {
        debug_assert!(ascii.is_ascii());
        for ch in ascii.chars() {
            if width == 0 {
                break 'outer;
            }

            let Some(cell) = buf.cell_mut((x, y)) else {
                break 'outer;
            };

            cell.set_char(ch).set_style(style);

            x += 1;
            width -= 1;
        }
    }

    Rect {
        x,
        y,
        width,
        ..line
    }
}

/// Prints char `n` times.
pub fn print_char_repeat(
    line: Rect,
    buf: &mut Buffer,
    ch: char,
    n: u8,
    style: impl Into<Style>,
) -> Rect {
    let style = style.into();
    let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
    let Rect {
        mut x,
        y,
        mut width,
        ..
    } = line;

    for _ in 0..n {
        if width == 0 {
            break;
        }

        match buf.cell_mut((x, y)) {
            Some(cell) => {
                cell.set_char(ch).set_style(style);
            }
            None => break,
        }

        width = width.saturating_sub(char_width);
        x += char_width;
    }

    Rect {
        x,
        y,
        width,
        height: line.height,
    }
}

/// Prints text `n` times.
pub fn print_text_repeat(
    line: Rect,
    buf: &mut Buffer,
    text: &str,
    n: u8,
    style: impl Into<Style>,
) -> Rect {
    let style = style.into();
    let Rect {
        mut x,
        y,
        mut width,
        ..
    } = line;

    for _ in 0..n {
        if width == 0 {
            break;
        }

        let (next_x, _) = buf.set_stringn(x, y, text, width as usize, style);
        width -= next_x - x;
        x = next_x;
    }

    Rect {
        x,
        y,
        width,
        height: line.height,
    }
}

/// Prints text and fills remaining empty cells with the given style.
pub fn print_text(
    line: Rect,
    buf: &mut Buffer,
    text: impl AsRef<str>,
    style: impl Into<Style>,
    fill: bool,
    alignment: Option<Alignment>,
) {
    let text = text.as_ref();
    let style = style.into();

    match alignment {
        Some(alignment) => {
            let text_width = unicode_width::UnicodeWidthStr::width(text);
            let Rect { x, y, .. } = align(
                Rect {
                    width: line.width.min(text_width as u16),
                    height: 1,
                    ..line
                },
                line,
                alignment,
            );
            let width = (line.x + line.width).saturating_sub(x);
            let (end_x, _) = buf.set_stringn(x, y, text, width as usize, style);

            if fill {
                let pre = x - line.x;
                for i in 0..pre {
                    if let Some(cell) = buf.cell_mut((line.x + i, y)) {
                        cell.set_style(style);
                    }
                }
                let post = line.width - (end_x - x);
                for i in 0..post {
                    match buf.cell_mut((end_x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
        None => {
            let Rect { x, y, width, .. } = line;
            let (end_x, _) = buf.set_stringn(x, y, text, width as usize, style);

            if fill {
                let remaining = width - (end_x - x);
                for i in 0..remaining {
                    match buf.cell_mut((end_x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
    }
}

/// Prints a collection of texts.
/// Fills remaining empty cells with the given style.
pub fn print_texts<'a>(
    line: Rect,
    buf: &mut Buffer,
    texts: impl IntoIterator<IntoIter: Clone, Item = &'a str>,
    style: impl Into<Style>,
    fill: bool,
    alignment: Option<Alignment>,
) {
    let style = style.into();
    let texts = texts.into_iter();

    match alignment {
        Some(alignment) => {
            let text_width = texts
                .clone()
                .map(|s| unicode_width::UnicodeWidthStr::width(s))
                .sum::<usize>();
            let Rect { mut x, y, .. } = align(
                Rect {
                    width: line.width.min(text_width as u16),
                    height: 1,
                    ..line
                },
                line,
                alignment,
            );
            let mut width = (line.x + line.width).saturating_sub(x);
            let mut end_x = 0;
            for text in texts {
                if width == 0 {
                    break;
                }
                (end_x, _) = buf.set_stringn(x, y, text, width as usize, style);
                width -= end_x - x;
                x = end_x;
            }

            if fill {
                let pre = x - line.x;
                for i in 0..pre {
                    if let Some(cell) = buf.cell_mut((line.x + i, y)) {
                        cell.set_style(style);
                    }
                }
                let post = line.width - (end_x - x);
                for i in 0..post {
                    match buf.cell_mut((end_x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
        None => {
            let Rect {
                mut x,
                y,
                mut width,
                ..
            } = line;

            for text in texts {
                if width == 0 {
                    break;
                }
                let (next_x, _) = buf.set_stringn(x, y, text, width as usize, style);
                width -= next_x - x;
                x = next_x;
            }

            if fill {
                for i in 0..width {
                    match buf.cell_mut((x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
    }
}

/// Prints a collection of texts with styles.
/// Fills remaining empty cells with the given fill style.
pub fn print_texts_with_styles<'a>(
    line: Rect,
    buf: &mut Buffer,
    texts: impl IntoIterator<IntoIter: Clone, Item = (&'a str, Style)>,
    fill_style: Option<Style>,
    alignment: Option<Alignment>,
) {
    let texts = texts.into_iter();

    match alignment {
        Some(alignment) => {
            let text_width = texts
                .clone()
                .map(|(s, _)| unicode_width::UnicodeWidthStr::width(s))
                .sum::<usize>();
            let Rect { mut x, y, .. } = align(
                Rect {
                    width: line.width.min(text_width as u16),
                    height: 1,
                    ..line
                },
                line,
                alignment,
            );
            let mut width = (line.x + line.width).saturating_sub(x);
            let mut end_x = 0;
            for (text, style) in texts {
                if width == 0 {
                    break;
                }
                (end_x, _) = buf.set_stringn(x, y, text, width as usize, style);
                width -= end_x - x;
                x = end_x;
            }

            if let Some(style) = fill_style {
                let pre = x - line.x;
                for i in 0..pre {
                    if let Some(cell) = buf.cell_mut((line.x + i, y)) {
                        cell.set_style(style);
                    }
                }
                let post = line.width - (end_x - x);
                for i in 0..post {
                    match buf.cell_mut((end_x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
        None => {
            let Rect {
                mut x,
                y,
                mut width,
                ..
            } = line;

            for (text, style) in texts {
                if width == 0 {
                    break;
                }
                let (next_x, _) = buf.set_stringn(x, y, text, width as usize, style);
                width -= next_x - x;
                x = next_x;
            }

            if let Some(style) = fill_style {
                for i in 0..width {
                    match buf.cell_mut((x + i, y)) {
                        Some(cell) => {
                            cell.set_style(style);
                        }
                        None => break,
                    }
                }
            }
        }
    }
}

/// Prints a collection of text segments with widths and spacing.
/// Fills remaining empty cells with the given style.
pub fn print_text_segments<'a>(
    line: Rect,
    buf: &mut Buffer,
    segments: impl IntoIterator<Item = (&'a str, u16, u16)>,
    style: impl Into<Style>,
) {
    let style = style.into();
    print_text_segments_with_styles(
        line,
        buf,
        segments.into_iter().map(|(s, w, g)| (s, w, g, style)),
        Some(style),
    );
}

/// Prints a collection of text segments with widths, spacing and styles.
/// Fills remaining empty cells with the given fill style.
pub fn print_text_segments_with_styles<'a>(
    line: Rect,
    buf: &mut Buffer,
    segments: impl IntoIterator<Item = (&'a str, u16, u16, Style)>,
    fill_style: Option<Style>,
) {
    let Rect { mut x, y, .. } = line;
    for (text, width, spacing, style) in segments {
        let text_width = width.saturating_sub(spacing);
        let (next_x, _) = buf.set_stringn(x, y, text, text_width as usize, style);
        let remaining = width - (next_x - x);
        x = next_x + remaining;

        if let Some(style) = fill_style {
            for i in 0..remaining {
                match buf.cell_mut((next_x + i, y)) {
                    Some(cell) => {
                        cell.set_style(style);
                    }
                    None => return,
                }
            }
        }
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

pub fn calculate_scroll(
    total_lines: usize,
    viewport_height: u16,
    selected: usize,
    offset: usize,
    margin_top: usize,
    margin_bottom: usize,
    padding_bottom: usize,
) -> usize {
    let height = viewport_height as usize;
    let max_offset = (total_lines + padding_bottom).saturating_sub(height);

    let available = height.saturating_sub(1);
    let margin_top = margin_top.min(available);
    let margin_bottom = margin_bottom.min(available - margin_top);

    let top_boundary = offset + margin_top;
    let bottom_boundary = offset + height.saturating_sub(margin_bottom + 1);

    if selected < top_boundary {
        // Scroll up
        offset.saturating_sub(top_boundary - selected)
    } else if selected > bottom_boundary {
        // Scroll down
        let delta = selected - bottom_boundary;
        (offset + delta).min(max_offset)
    } else {
        // No scroll
        offset
    }
}
