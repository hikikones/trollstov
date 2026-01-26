use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use unicode_segmentation::UnicodeSegmentation;

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

    pub fn clear(&mut self) {
        self.text.clear();
        self.styles.clear();
        self.total_width = 0;
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }
        let Rect { mut x, mut y, .. } = area;
        let max_width = area.width as usize;
        let mut start = 0;
        let mut current_width = 0;
        let mut remaining_width = max_width;
        // let mut x = 0;
        for (style, len, width) in self.styles.iter().copied() {
            let end = start + len;
            let text = &self.text[start..end];
            // if current_width + width <= max_width {
            //     // buf.render string
            // } else {
            //     // no fit, split up
            // }
            // remaining_width = remaining_width.saturating_sub(width);
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

            // if current_width + width > max_width {
            //     break;
            // }

            // current_width += width;
            // x +=

            //todo render
            // for g in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
            //     let w = unicode_width::UnicodeWidthStr::width(g);
            //     cw += w;
            //     if cw > max_width {
            //         return;
            //     }

            //     buf[(0, 0)].set_style(style).set_symbol(symbol);
            //     buf.set_stringn(x, y, string, max_width, style)
            // }
            start = end;
        }
    }
}
