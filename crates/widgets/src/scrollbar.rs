use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::{Color, Style},
};

pub struct Scrollbar {
    data: ScrollbarData,
    colors: ScrollbarColors,
}

impl Scrollbar {
    pub const fn new(data: ScrollbarData) -> Self {
        Self {
            data,
            colors: ScrollbarColors::DEFAULT,
        }
    }

    pub const fn with_colors(mut self, colors: ScrollbarColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() || self.data.total_items == 0 {
            return;
        }

        let ScrollbarData {
            viewport_height,
            current_scroll,
            total_items,
        } = self.data;

        let visible = viewport_height as f32 / total_items as f32;
        let size = ((visible * area.height as f32).floor() as u16).max(1);
        let progress = (current_scroll as f32
            / total_items.saturating_sub(viewport_height as usize) as f32)
            .min(1.0);
        let range = area.height.saturating_sub(size);
        let start = (progress * range as f32).floor() as u16;
        let end = start + size;

        let thumb_style = Style::new().fg(self.colors.thumb);
        let Rect { x, mut y, .. } = area;

        match self.colors.track {
            // Render both track and thumb
            Some(track_color) => {
                let track_style = Style::new().fg(track_color);
                for i in 0..area.height {
                    match buf.cell_mut((x, y)) {
                        Some(cell) => {
                            let (symbol, style) = if i >= start && i < end {
                                ("┃", thumb_style)
                            } else {
                                ("│", track_style)
                            };
                            cell.set_symbol(symbol).set_style(style);
                        }
                        None => return,
                    }
                    y += 1;
                }
            }
            // Render only thumb
            None => {
                for i in 0..area.height {
                    match buf.cell_mut((x, y)) {
                        Some(cell) => {
                            if i >= start && i < end {
                                cell.set_symbol("│").set_style(thumb_style);
                            }
                        }
                        None => return,
                    }
                    y += 1;
                }
            }
        }
    }

    pub const fn calculate_scroll(data: ScrollData) -> usize {
        Self::calculate_scroll_with_margins(
            data,
            ScrollMargins {
                margin_top: 0,
                margin_bottom: 0,
                padding_bottom: 0,
            },
        )
    }

    pub const fn calculate_scroll_with_margins(data: ScrollData, margins: ScrollMargins) -> usize {
        let ScrollData {
            total_lines,
            viewport_height,
            current_index,
            current_scroll,
        } = data;

        let ScrollMargins {
            margin_top,
            margin_bottom,
            padding_bottom,
        } = margins;

        const fn min(a: usize, b: usize) -> usize {
            if a < b { a } else { b }
        }

        let height = viewport_height as usize;
        let max_offset = (total_lines + padding_bottom as usize).saturating_sub(height);

        let available = height.saturating_sub(1);
        let margin_top = min(margin_top as usize, available);
        let margin_bottom = min(margin_bottom as usize, available - margin_top);

        let top_boundary = current_scroll + margin_top;
        let bottom_boundary = current_scroll + height.saturating_sub(margin_bottom + 1);

        if current_index < top_boundary {
            // Scroll up
            current_scroll.saturating_sub(top_boundary - current_index)
        } else if current_index > bottom_boundary {
            // Scroll down
            let delta = current_index - bottom_boundary;
            min(current_scroll + delta, max_offset)
        } else {
            // No scroll
            current_scroll
        }
    }

    pub const fn is_scrollable(data: ScrollableData) -> bool {
        data.total_lines > data.viewport_size.height as usize
            && data.viewport_size.width > data.min_width
    }

    pub const fn make_scroll_area(area: &mut Rect) -> Rect {
        Self::make_scroll_area_with_margin(area, 1)
    }

    pub const fn make_scroll_area_with_margin(area: &mut Rect, margin: u16) -> Rect {
        let scroll_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            width: 1,
            ..*area
        };
        area.width = area.width.saturating_sub(1 + margin);
        scroll_area
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollData {
    pub current_index: usize,
    pub current_scroll: usize,
    pub total_lines: usize,
    pub viewport_height: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollableData {
    pub total_lines: usize,
    pub viewport_size: Size,
    pub min_width: u16,
}

impl ScrollableData {
    pub const fn new(total_lines: usize, viewport_size: Size) -> Self {
        Self {
            total_lines,
            viewport_size,
            min_width: 15,
        }
    }

    pub const fn with_min_width(mut self, min_width: u16) -> Self {
        self.min_width = min_width;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollMargins {
    pub margin_top: u16,
    pub margin_bottom: u16,
    pub padding_bottom: u16,
}

impl ScrollMargins {
    pub const ZERO: Self = Self {
        margin_top: 0,
        margin_bottom: 0,
        padding_bottom: 0,
    };

    pub const fn new(top: u16, bottom: u16, padding_bottom: u16) -> Self {
        Self {
            margin_top: top,
            margin_bottom: bottom,
            padding_bottom,
        }
    }

    pub const fn all(v: u16) -> Self {
        Self::new(v, v, v)
    }

    pub const fn vertical(v: u16) -> Self {
        Self::new(v, v, 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarData {
    pub current_scroll: usize,
    pub total_items: usize,
    pub viewport_height: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarColors {
    pub thumb: Color,
    pub track: Option<Color>,
}

impl ScrollbarColors {
    pub const DEFAULT: Self = Self {
        thumb: Color::Indexed(240),
        track: None,
    };

    pub const fn new(thumb: Color, track: Option<Color>) -> Self {
        Self { thumb, track }
    }
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self::DEFAULT
    }
}
