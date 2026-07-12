use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Constraint, Layout, Rect, Size},
    widgets::Padding,
};

use crate::{
    RectExt, ScrollData, ScrollMargins, ScrollableData, Scrollbar, ScrollbarColors, ScrollbarData,
};

pub struct Table<const N: usize> {
    index: usize,
    selector: Option<usize>,
    scroll: usize,
    layout: TableLayout<N>,
    options: TableOptions,
    colors: TableColors,
    last_height: u16,
    list_height: u16,
    len: usize,
}

pub struct TableLayout<const N: usize> {
    constraints: [Constraint; N],
    spacing: u16,
}

impl<const N: usize> TableLayout<N> {
    pub const fn new(constraints: [Constraint; N], spacing: u16) -> Self {
        Self {
            constraints,
            spacing,
        }
    }

    fn areas(&self, area: Rect) -> [Rect; N] {
        Layout::horizontal(self.constraints)
            .spacing(self.spacing)
            .areas(area)
    }
}

pub enum TableMove {
    Up(usize),
    Down(usize),
    PageUp,
    PageDown,
    Start,
    End,
    Custom(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TableItem {
    Selected,
    Selection,
    Normal,
}

impl<const N: usize> Table<N> {
    pub const fn new(layout: TableLayout<N>) -> Self {
        Self {
            index: 0,
            selector: None,
            scroll: 0,
            layout,
            options: TableOptions::new(),
            colors: TableColors::new(),
            last_height: 0,
            list_height: 0,
            len: 0,
        }
    }

    pub const fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    pub const fn with_scrolloff(mut self, margins: ScrollMargins) -> Self {
        self.set_scrolloff(margins);
        self
    }

    pub const fn with_padding(mut self, padding: Padding) -> Self {
        self.set_padding(padding);
        self
    }

    pub const fn with_scrollbar(mut self, margin: u16) -> Self {
        self.options.scrollbar = true;
        self.options.scrollbar_margin = margin;
        self
    }

    pub const fn with_colors(mut self, colors: TableColors) -> Self {
        self.set_colors(colors);
        self
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn selector(&self) -> Option<usize> {
        self.selector
    }

    pub const fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        self.selector
            .and_then(|selector| match self.index.cmp(&selector) {
                std::cmp::Ordering::Less => Some((self.index + 1)..(selector + 1)),
                std::cmp::Ordering::Greater => Some(selector..self.index),
                std::cmp::Ordering::Equal => None,
            })
    }

    pub fn selection_inclusive(&self) -> std::ops::RangeInclusive<usize> {
        self.selector
            .map(|selector| {
                if self.index < selector {
                    self.index..=selector
                } else {
                    selector..=self.index
                }
            })
            .unwrap_or(self.index..=self.index)
    }

    pub const fn set_index(&mut self, i: usize) -> &mut Self {
        self.index = i;
        self
    }

    pub const fn set_selector(&mut self, s: Option<usize>) -> &mut Self {
        self.selector = s;
        self
    }

    pub const fn set_scrolloff(&mut self, margins: ScrollMargins) -> &mut Self {
        self.options.scrolloff = margins;
        self
    }

    pub const fn set_padding(&mut self, padding: Padding) -> &mut Self {
        self.options.padding = padding;
        self
    }

    pub const fn set_scrollbar(&mut self, enabled: bool) -> &mut Self {
        self.options.scrollbar = enabled;
        self
    }

    pub const fn set_colors(&mut self, colors: TableColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub fn move_index(&mut self, lm: TableMove, shift: bool) -> bool {
        match lm {
            TableMove::Up(n) => self.set_index_and_selector(self.index.saturating_sub(n), shift),
            TableMove::Down(n) => self.set_index_and_selector(self.index + n, shift),
            TableMove::PageUp => {
                let n = self.list_height as usize;
                self.set_index_and_selector(self.index.saturating_sub(n), shift)
            }
            TableMove::PageDown => {
                let n = self.list_height as usize;
                self.set_index_and_selector(self.index + n, shift)
            }
            TableMove::Start => self.set_index_and_selector(0, shift),
            TableMove::End => self.set_index_and_selector(usize::MAX, shift),
            TableMove::Custom(i) => self.set_index_and_selector(i, shift),
        }
    }

    pub fn move_selection_up(&mut self) -> bool {
        let Some(selector) = self.selector else {
            return self.set_index_and_selector(self.index.saturating_sub(1), false);
        };

        let index = self.index;
        let i = index.saturating_sub(1);
        let s = selector.saturating_sub(1);

        if i == index || s == selector {
            return false;
        }

        self.index = i;
        self.selector = Some(s);
        true
    }

    pub fn move_selection_down(&mut self) -> bool {
        let Some(selector) = self.selector else {
            return self.set_index_and_selector(self.index + 1, false);
        };

        let index = self.index;
        let max_index = self.len.saturating_sub(1);
        let i = usize::min(index + 1, max_index);
        let s = usize::min(selector + 1, max_index);

        if i == index || s == selector {
            return false;
        }

        self.index = i;
        self.selector = Some(s);
        true
    }

    pub fn select_all(&mut self) -> bool {
        let old_index = self.index;
        let old_selector = self.selector;

        self.index = 0;
        self.selector = Some(self.len.saturating_sub(1));
        self.selector.take_if(|s| *s == self.index);

        old_index != self.index || old_selector != self.selector
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Down => self.move_index(TableMove::Down(1), shift),
            KeyCode::Up => self.move_index(TableMove::Up(1), shift),
            KeyCode::PageDown => self.move_index(TableMove::PageDown, shift),
            KeyCode::PageUp => self.move_index(TableMove::PageUp, shift),
            KeyCode::End => self.move_index(TableMove::End, shift),
            KeyCode::Home => self.move_index(TableMove::Start, shift),
            KeyCode::Char('a') => {
                if ctrl {
                    self.select_all()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub const fn reset(&mut self) {
        self.index = 0;
        self.scroll = 0;
        self.selector = None;
    }

    pub fn render<T>(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        items: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
        render_header: impl FnOnce(Rect, &mut Buffer, [Rect; N]),
        mut render_row: impl FnMut(Rect, &mut Buffer, [Rect; N], T, TableItem),
    ) {
        let mut inner = area.inner_padding(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let items = items.into_iter();
        self.len = items.len();

        let header_y = inner.y;

        let scroll_area = if self.is_scrollable(inner.as_size()) {
            let scroll_area =
                Scrollbar::make_scroll_area_with_margin(&mut area, self.options.scrollbar_margin);
            inner.width = inner
                .width
                .saturating_sub(scroll_area.width + self.options.scrollbar_margin);
            Some(scroll_area)
        } else {
            None
        };

        inner.shrink_down(1);

        // Render header
        let header = Rect {
            y: header_y,
            height: 1,
            ..inner
        };
        let mut areas = self.layout.areas(header);
        render_header(header, buf, areas);

        // Update state
        self.list_height = inner.height;
        self.clamp_index_and_selector();
        self.update_scroll(area.height);
        self.last_height = area.height;

        // Render table
        let selection = self.selection_inclusive();
        let mut row = Rect { height: 1, ..inner };

        items
            .enumerate()
            .skip(self.scroll)
            .take(inner.height as usize)
            .for_each(|(i, item)| {
                let list_item = if i == self.index {
                    TableItem::Selected
                } else if selection.contains(&i) {
                    TableItem::Selection
                } else {
                    TableItem::Normal
                };

                areas.iter_mut().for_each(|a| a.y += 1);
                render_row(row, buf, areas, item, list_item);
                row.y += 1;
            });

        // Render scrollbar
        if let Some(scroll_area) = scroll_area {
            Scrollbar::new(ScrollbarData {
                viewport_height: inner.height,
                current_scroll: self.scroll,
                total_items: self.len,
            })
            .with_colors(self.colors.scrollbar)
            .render(scroll_area, buf);
        }
    }

    fn set_index_and_selector(&mut self, i: usize, shift: bool) -> bool {
        let old_index = self.index;
        let old_selector = self.selector;

        if shift {
            if self.selector.is_none() {
                self.selector = Some(self.index);
            }
        } else {
            self.selector = None;
        }

        self.index = usize::min(i, self.len.saturating_sub(1));
        self.selector.take_if(|s| *s == self.index);

        old_index != self.index || old_selector != self.selector
    }

    const fn is_scrollable(&self, list_size: Size) -> bool {
        self.options.scrollbar && Scrollbar::is_scrollable(ScrollableData::new(self.len, list_size))
    }

    fn clamp_index_and_selector(&mut self) {
        let max_idx = self.len.saturating_sub(1);
        self.index = self.index.min(max_idx);
        self.selector = self.selector.map(|selector| selector.min(max_idx));
    }

    const fn update_scroll(&mut self, area_height: u16) {
        let scroll = if self.last_height != area_height {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = Scrollbar::calculate_scroll_with_margins(
            ScrollData {
                current_index: self.index,
                current_scroll: scroll,
                total_lines: self.len,
                viewport_height: self.list_height,
            },
            self.options.scrolloff,
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableOptions {
    pub scrolloff: ScrollMargins,
    pub padding: Padding,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
}

impl TableOptions {
    pub const fn new() -> Self {
        Self {
            scrolloff: ScrollMargins::ZERO,
            padding: Padding::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
        }
    }
}

impl Default for TableOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableColors {
    pub scrollbar: ScrollbarColors,
}

impl TableColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for TableColors {
    fn default() -> Self {
        Self::new()
    }
}
