use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
    style::{Color, Style},
};

use super::utils;

pub struct List {
    index: usize,
    selector: Option<usize>,
    scroll: usize,
    margin_top: usize,
    margin_bottom: usize,
    padding_bottom: usize,
    thumb_color: Color,
    track_color: Option<Color>,
    len: usize,
    height: u16,
}

pub enum ListMove {
    Up,
    Down,
    PageUp,
    PageDown,
    Start,
    End,
    Custom(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListItem {
    Selected,
    Selection,
    Normal,
}

impl List {
    pub const fn new() -> Self {
        Self {
            index: 0,
            selector: None,
            scroll: 0,
            margin_top: 0,
            margin_bottom: 0,
            padding_bottom: 0,
            thumb_color: Color::Gray,
            track_color: Some(Color::DarkGray),
            len: 0,
            height: 0,
        }
    }

    pub const fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    pub const fn set_colors(&mut self, thumb: Color, track: Option<Color>) -> &mut Self {
        self.thumb_color = thumb;
        self.track_color = track;
        self
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn selection(&self) -> std::ops::RangeInclusive<usize> {
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

    pub const fn set_margins(&mut self, margin_top: usize, margin_bottom: usize) -> &mut Self {
        self.margin_top = margin_top;
        self.margin_bottom = margin_bottom;
        self
    }

    pub const fn set_padding(&mut self, padding_bottom: usize) -> &mut Self {
        self.padding_bottom = padding_bottom;
        self
    }

    pub fn move_index(&mut self, lm: ListMove, shift: bool) -> bool {
        let old_index = self.index;
        let old_selector = self.selector;

        if shift {
            if self.selector.is_none() {
                self.selector = Some(self.index);
            }
        } else {
            self.selector = None;
        }

        match lm {
            ListMove::Up => {
                self.index = self.index.saturating_sub(1);
            }
            ListMove::Down => {
                self.index = usize::min(self.index + 1, self.len.saturating_sub(1));
            }
            ListMove::PageUp => self.index = self.index.saturating_sub(self.height as usize),
            ListMove::PageDown => {
                self.index = usize::min(
                    self.index + self.height as usize,
                    self.len.saturating_sub(1),
                );
            }
            ListMove::Start => {
                self.index = 0;
            }
            ListMove::End => {
                self.index = self.len.saturating_sub(1);
            }
            ListMove::Custom(i) => self.index = i,
        }

        self.selector.take_if(|s| *s == self.index);

        old_index != self.index || old_selector != self.selector
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
            KeyCode::Down => self.move_index(ListMove::Down, shift),
            KeyCode::Up => self.move_index(ListMove::Up, shift),
            KeyCode::PageDown => self.move_index(ListMove::PageDown, shift),
            KeyCode::PageUp => self.move_index(ListMove::PageUp, shift),
            KeyCode::End => self.move_index(ListMove::End, shift),
            KeyCode::Home => self.move_index(ListMove::Start, shift),
            KeyCode::Char(c) => match c {
                'a' => {
                    if ctrl {
                        self.select_all()
                    } else {
                        false
                    }
                }
                _ => false,
            },
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
        items: impl ExactSizeIterator<Item = T>,
        mut render_line: impl FnMut(Rect, &mut Buffer, T, ListItem),
    ) {
        // Make sure index and selector is not out of bounds
        let max_idx = items.len().saturating_sub(1);
        self.index = self.index.min(max_idx);
        self.selector = self.selector.map(|selector| selector.min(max_idx));

        // Determine scroll
        let scroll = if self.height != area.height {
            // Refresh scroll on window resize
            0
        } else {
            self.scroll
        };
        self.scroll = utils::calculate_scroll(
            items.len(),
            area.height,
            self.index,
            scroll,
            self.margin_top,
            self.margin_bottom,
            self.padding_bottom,
        );

        self.len = items.len();
        self.height = area.height;

        // Render
        let height = area.height as usize;
        let scrollable = items.len() > height;

        if scrollable {
            let scrollbar = Rect {
                x: area.x + area.width,
                width: 1,
                ..area
            };
            area.width = area.width.saturating_sub(1);
            render_scrollbar(
                scrollbar,
                buf,
                items.len(),
                self.scroll,
                self.thumb_color,
                self.track_color,
            );
        }

        let selection = self.selection();
        let mut line = Rect { height: 1, ..area };

        items
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, item)| {
                let list_item = if i == self.index {
                    ListItem::Selected
                } else if selection.contains(&i) {
                    ListItem::Selection
                } else {
                    ListItem::Normal
                };

                render_line(line, buf, item, list_item);

                line.y += 1;
            });
    }
}

fn render_scrollbar(
    vertical_line: Rect,
    buf: &mut Buffer,
    total_items: usize,
    current_scroll: usize,
    thumb_color: Color,
    track_color: Option<Color>,
) {
    let height = vertical_line.height as usize;
    if total_items == 0 || height == 0 {
        return;
    }

    let visible = height as f32 / total_items as f32;
    let size = ((visible * height as f32).round() as usize).max(1);
    let progress = (current_scroll as f32 / total_items.saturating_sub(height) as f32).min(1.0);
    let range = height.saturating_sub(size);
    let start = (progress * range as f32).round() as usize;
    let end = start + size;

    let thumb_style = Style::new().fg(thumb_color);
    let Rect { x, mut y, .. } = vertical_line;

    match track_color {
        Some(track_color) => {
            let track_style = Style::new().fg(track_color);
            for i in 0..height {
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
        None => {
            for i in 0..height {
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
