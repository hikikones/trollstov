use std::cmp::Ordering;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
};

pub struct List {
    index: usize,
    selector: Option<usize>,
    scroll: usize,
    offset: usize,
    old_index: usize,
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

impl List {
    pub const fn new() -> Self {
        Self {
            index: 0,
            selector: None,
            scroll: 0,
            offset: 0,
            old_index: 0,
            len: 0,
            height: 0,
        }
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

    pub const fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub fn move_index(&mut self, lm: ListMove, shift: bool) -> bool {
        self.old_index = self.index;
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

        self.old_index != self.index || old_selector != self.selector
    }

    pub fn select_all(&mut self) -> bool {
        self.old_index = self.index;
        let old_selector = self.selector;

        self.index = 0;
        self.selector = Some(self.len.saturating_sub(1));
        self.selector.take_if(|s| *s == self.index);

        self.old_index != self.index || old_selector != self.selector
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

    pub fn render<T>(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        items: impl ExactSizeIterator<Item = T>,
        mut render_line: impl FnMut(Rect, &mut Buffer, T, bool, bool),
    ) {
        // Make sure index and selector is not out of bounds
        let max_idx = items.len().saturating_sub(1);
        self.index = self.index.min(max_idx);
        self.selector = self.selector.map(|selector| selector.min(max_idx));

        // Determine scroll
        let offset = self.offset;
        let index = self.index;
        let scroll = self.scroll;
        let height = (area.height as usize).saturating_sub(offset);

        if self.height != area.height {
            // Fixes window resizing when going from small to big,
            // leaving empty space when scroll stays the same at the end
            let max_scroll = items.len().saturating_sub(height + offset);
            self.scroll = self.scroll.min(max_scroll);
        } else {
            match index.cmp(&self.old_index) {
                Ordering::Greater => {
                    // Scroll down
                    if index > scroll {
                        let diff = index - scroll;
                        if diff >= height {
                            let max_scroll = items.len().saturating_sub(height + offset);
                            let new_scroll = scroll + diff - height.saturating_sub(1);
                            self.scroll = new_scroll.min(max_scroll);
                        }
                    }
                }
                Ordering::Less => {
                    // Scroll up
                    if scroll + offset > index {
                        let diff = scroll + offset - index;
                        self.scroll = scroll.saturating_sub(diff);
                    }
                }
                Ordering::Equal => {
                    // No scroll
                }
            }
        }

        self.len = items.len();
        self.height = area.height;

        // Render
        let selection = self.selection();
        let mut line = Rect { height: 1, ..area };

        items
            .enumerate()
            .skip(self.scroll)
            .take(area.height as usize)
            .for_each(|(i, item)| {
                let is_index = i == self.index;
                let is_selected = i >= *selection.start() && i <= *selection.end();

                render_line(line, buf, item, is_index, is_selected);

                line.y += 1;
            });
    }
}
