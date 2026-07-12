use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{Rect, Size},
    widgets::Padding,
};

use crate::{RectExt, ScrollData, ScrollableData, Scrollbar, ScrollbarColors, ScrollbarData};

pub struct TokenList {
    index: usize,
    index_col: u16,
    index_row: u16,
    scroll: u16,
    total_lines: u16,
    total_items: usize,
    list_width: u16,
    last_size: Size,
    options: TokenListOptions,
    colors: TokenListColors,
}

pub trait TokenItem {
    fn width(&self) -> u16;
}

impl TokenList {
    pub const fn new() -> Self {
        Self {
            index: 0,
            index_col: 0,
            index_row: 0,
            scroll: 0,
            total_lines: 0,
            total_items: 0,
            list_width: 0,
            last_size: Size::ZERO,
            options: TokenListOptions::new(),
            colors: TokenListColors::new(),
        }
    }

    pub const fn with_gap(mut self, gap: u16) -> Self {
        self.options.gap = gap;
        self
    }

    pub const fn with_padding(mut self, padding: Padding) -> Self {
        self.options.padding = padding;
        self
    }

    pub const fn with_scrollbar(mut self) -> Self {
        self.options.scrollbar = true;
        self
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn set_index(&mut self, i: usize) -> &mut Self {
        self.index = i;
        self
    }

    pub const fn set_colors(&mut self, colors: TokenListColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub fn input<T: TokenItem>(
        &mut self,
        key: KeyCode,
        items: impl IntoIterator<Item = T>,
    ) -> bool {
        let old_index = self.index;

        match key {
            KeyCode::Right => {
                self.index = (self.index + 1).min(self.total_items.saturating_sub(1));
            }
            KeyCode::Left => {
                self.index = self.index.saturating_sub(1);
            }
            KeyCode::Down => {
                self.index = if self.index_row == self.total_lines.saturating_sub(1) {
                    self.total_items.saturating_sub(1)
                } else {
                    let (mut next_index, mut distance) = (0, u16::MAX);
                    for (i, x, y, _) in
                        iter_items_in_col_row(self.list_width, self.options.gap, items)
                            .skip(self.index + 1)
                    {
                        if y == self.index_row + 1 {
                            let d = self.index_col.abs_diff(x);
                            if d <= distance {
                                next_index = i;
                                distance = d;
                            }
                        } else if y > self.index_row + 1 {
                            break;
                        }
                    }
                    next_index
                };
            }
            KeyCode::Up => {
                self.index = if self.index_row == 0 {
                    0
                } else {
                    let (mut next_index, mut distance) = (0, u16::MAX);
                    for (i, x, y, _) in
                        iter_items_in_col_row(self.list_width, self.options.gap, items)
                    {
                        if y == self.index_row.saturating_sub(1) {
                            let d = self.index_col.abs_diff(x);
                            if d <= distance {
                                next_index = i;
                                distance = d;
                            }
                        } else if y >= self.index_row {
                            break;
                        }
                    }
                    next_index
                };
            }
            KeyCode::Home => {
                self.index = 0;
            }
            KeyCode::End => {
                self.index = self.total_items.saturating_sub(1);
            }
            _ => {}
        }

        self.index != old_index
    }

    pub fn render<T: TokenItem>(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        items: impl IntoIterator<Item = T, IntoIter: Clone>,
        mut render_item: impl FnMut(Rect, &mut Buffer, T, bool),
    ) {
        let mut inner = area.inner_padding(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let items = items.into_iter();

        self.process_items(inner, items.clone());

        let scroll_area = if self.is_scrollable(inner.as_size()) {
            let scroll_area =
                Scrollbar::make_scroll_area_with_margin(&mut area, self.options.scrollbar_margin);
            inner.width = inner
                .width
                .saturating_sub(scroll_area.width + self.options.scrollbar_margin);
            self.process_items(inner, items.clone());
            Some(scroll_area)
        } else {
            None
        };

        self.update_scroll(area.as_size(), inner.height);

        self.last_size = area.as_size();
        self.list_width = inner.width;

        // Render list
        for (i, x, y, item) in iter_items_in_col_row(inner.width, self.options.gap, items) {
            if y >= inner.height + self.scroll {
                break;
            }

            if y >= self.scroll {
                let token = Rect {
                    x: inner.x + x,
                    y: inner.y + y.saturating_sub(self.scroll),
                    width: item.width().min(inner.width.saturating_sub(x)),
                    height: 1,
                };
                render_item(token, buf, item, self.index == i);
            }
        }

        // Render scrollbar
        if let Some(scroll_area) = scroll_area {
            Scrollbar::new(ScrollbarData {
                viewport_height: inner.height,
                current_scroll: self.scroll as usize,
                total_items: self.total_lines as usize,
            })
            .with_colors(self.colors.scrollbar)
            .render(scroll_area, buf);
        }
    }

    fn process_items<T: TokenItem>(
        &mut self,
        area: Rect,
        items: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.total_items = 0;

        for (i, x, y, _) in iter_items_in_col_row(area.width, self.options.gap, items) {
            if self.index == i {
                self.index_col = x;
                self.index_row = y;
            }
            self.total_items += 1;
            self.total_lines = y + 1;
        }

        self
    }

    const fn is_scrollable(&self, list_size: Size) -> bool {
        self.options.scrollbar
            && Scrollbar::is_scrollable(ScrollableData::new(self.total_lines as usize, list_size))
    }

    fn update_scroll(&mut self, area_size: Size, list_height: u16) {
        let scroll = if self.last_size != area_size {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scrollbar::calculate_scroll(ScrollData {
            current_index: self.index_row as usize,
            current_scroll: scroll as usize,
            total_lines: self.total_lines as usize,
            viewport_height: list_height,
        }) as u16;
    }
}

fn iter_items_in_col_row<T: TokenItem>(
    max_width: u16,
    item_gap: u16,
    items: impl IntoIterator<Item = T>,
) -> impl Iterator<Item = (usize, u16, u16, T)> {
    let (mut x, mut y) = (0, 0);
    items.into_iter().enumerate().map(move |(i, item)| {
        let item_width = item.width();
        if x + item_width > max_width {
            x = 0;
            if i > 0 {
                y += 1;
            }
        }

        let (col, row) = (x, y);

        x += item_width + item_gap;

        (i, col, row, item)
    })
}

#[derive(Debug, Clone, Copy)]
pub struct TokenListOptions {
    pub gap: u16,
    pub padding: Padding,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
}

impl TokenListOptions {
    pub const fn new() -> Self {
        Self {
            gap: 2,
            padding: Padding::uniform(1),
            scrollbar: true,
            scrollbar_margin: 1,
        }
    }
}

impl Default for TokenListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TokenListColors {
    pub scrollbar: ScrollbarColors,
}

impl TokenListColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for TokenListColors {
    fn default() -> Self {
        Self::new()
    }
}
