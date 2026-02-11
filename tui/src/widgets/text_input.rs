use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct TextInput {
    input: String,
    placeholder: &'static str,
    cursor_index: usize,
    cursor_column: usize,
    selection_start: Option<usize>,
    scroll: usize,
    cursor_style: Style,
    selector_style: Style,
    placeholder_style: Style,
    spans: Vec<Span<'static>>,
}

pub enum CursorMove {
    Forward,
    Back,
    Up,
    Down,
    Start,
    End,
}

pub enum CursorDelete {
    Forward,
    Back,
    _Selection,
}

impl TextInput {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            placeholder: "",
            cursor_index: 0,
            cursor_column: 0,
            selection_start: None,
            scroll: 0,
            cursor_style: Style::new().bg(Color::White).fg(Color::Black),
            selector_style: Style::new().bg(Color::DarkGray).fg(Color::Gray),
            placeholder_style: Style::new().fg(Color::DarkGray).italic(),
            spans: Vec::new(),
        }
    }

    pub const fn with_placeholder(mut self, s: &'static str) -> Self {
        self.placeholder = s;
        self
    }

    pub const fn with_styles(mut self, cursor: Style, selector: Style, placeholder: Style) -> Self {
        self.cursor_style = cursor;
        self.selector_style = selector;
        self.placeholder_style = placeholder;
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    pub const fn as_str(&self) -> &str {
        self.input.as_str()
    }

    pub fn input(&mut self, key_pressed: KeyCode, key_modifiers: KeyModifiers) -> bool {
        let ctrl = key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_modifiers.contains(KeyModifiers::SHIFT);

        match key_pressed {
            KeyCode::Right => return self.move_cursor(CursorMove::Forward, shift),
            KeyCode::Left => return self.move_cursor(CursorMove::Back, shift),
            KeyCode::Up => return self.move_cursor(CursorMove::Up, shift),
            KeyCode::Down => return self.move_cursor(CursorMove::Down, shift),
            KeyCode::Home => return self.move_cursor(CursorMove::Start, shift),
            KeyCode::End => return self.move_cursor(CursorMove::End, shift),
            KeyCode::Backspace => return self.delete(CursorDelete::Back),
            KeyCode::Delete => return self.delete(CursorDelete::Forward),
            KeyCode::Char(c) => match c {
                'a' => {
                    if ctrl {
                        return self.select_all();
                    }

                    self.push_char(c);
                    return true;
                }
                'c' => {
                    if ctrl {
                        if let Some(selector) = self.selection_start {
                            if let Some(range) = self.get_selection_range(selector) {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(&self.input[range]);
                                }
                            }
                        }
                    } else {
                        self.push_char(c);
                        return true;
                    }
                }
                'v' => {
                    if ctrl {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(s) = clipboard.get_text() {
                                self.push_str(&s);
                                return true;
                            }
                        }
                    } else {
                        self.push_char(c);
                        return true;
                    }
                }
                _ => {
                    self.push_char(c);
                    return true;
                }
            },
            _ => {}
        }

        false
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(start) = self.selection_start.take() {
            self.delete_selection(start);
        }
        let c = if c.is_whitespace() { ' ' } else { c };
        self.input.insert(self.cursor_index, c);
        self.cursor_index += c.len_utf8();
    }

    pub fn push_str(&mut self, s: &str) {
        if let Some(start) = self.selection_start.take() {
            self.delete_selection(start);
        }
        s.graphemes(true)
            .map(|g| {
                if g.chars().any(|c| c.is_whitespace()) {
                    " "
                } else {
                    g
                }
            })
            .for_each(|g| {
                self.input.insert_str(self.cursor_index, g);
                self.cursor_index += g.len();
            });
    }

    pub fn move_cursor(&mut self, cm: CursorMove, shift: bool) -> bool {
        let (old_cursor, old_selector) = (self.cursor_index, self.selection_start);

        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_index);
            }
        } else {
            self.selection_start = None;
        }

        match cm {
            CursorMove::Forward => {
                if let Some(g) = self.input[self.cursor_index..].graphemes(true).next() {
                    self.cursor_index += g.len();
                }
            }
            CursorMove::Back => {
                if let Some(g) = self.input[..self.cursor_index].graphemes(true).rev().next() {
                    self.cursor_index -= g.len();
                }
            }
            CursorMove::Up | CursorMove::Start => {
                self.cursor_index = 0;
            }
            CursorMove::Down | CursorMove::End => {
                self.cursor_index = self.input.len();
            }
        }

        self.selection_start.take_if(|s| *s == self.cursor_index);

        self.cursor_index != old_cursor || self.selection_start != old_selector
    }

    pub fn select_all(&mut self) -> bool {
        let (old_cursor, old_selector) = (self.cursor_index, self.selection_start);

        self.cursor_index = self.input.len();
        self.selection_start = Some(0);

        self.cursor_index != old_cursor || self.selection_start != old_selector
    }

    pub fn delete(&mut self, cd: CursorDelete) -> bool {
        match cd {
            CursorDelete::Forward => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => match self.input[self.cursor_index..].graphemes(true).next() {
                    Some(g) => {
                        self.input
                            .replace_range(self.cursor_index..self.cursor_index + g.len(), "");
                        true
                    }
                    None => false,
                },
            },
            CursorDelete::Back => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => match self.input[..self.cursor_index].graphemes(true).rev().next() {
                    Some(g) => {
                        self.cursor_index -= g.len();
                        self.input
                            .replace_range(self.cursor_index..self.cursor_index + g.len(), "");
                        true
                    }
                    None => false,
                },
            },
            CursorDelete::_Selection => match self.selection_start.take() {
                Some(selector) => self.delete_selection(selector),
                None => false,
            },
        }
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_index = 0;
        self.cursor_column = 0;
        self.selection_start = None;
        self.scroll = 0;
        self.spans.clear();
    }

    fn get_selection_range(&self, selector: usize) -> Option<std::ops::Range<usize>> {
        match self.cursor_index.cmp(&selector) {
            std::cmp::Ordering::Less => Some(self.cursor_index..selector),
            std::cmp::Ordering::Greater => Some(selector..self.cursor_index),
            std::cmp::Ordering::Equal => None,
        }
    }

    fn delete_selection(&mut self, selector: usize) -> bool {
        let Some(range) = self.get_selection_range(selector) else {
            return false;
        };
        self.cursor_index = range.start;
        self.input.replace_range(range, "");
        true
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.spans.clear();
        self.cursor_column = 0;

        let mut total_width = 0;
        let input_len = self.input.len();
        let selection_start = self
            .cursor_index
            .min(self.selection_start.unwrap_or(self.cursor_index));
        let selection_end = self
            .selection_start
            .unwrap_or(self.cursor_index)
            .max(self.cursor_index);
        let cursor_style = self.cursor_style;
        let selector_style = self.selector_style;

        // Get total input width and update scroll
        let total_width = unicode_width::UnicodeWidthStr::width(self.input.as_str());
        self.scroll = calculate_scroll(
            total_width,
            area.width,
            self.cursor_index,
            self.scroll,
            0,
            0,
            0,
        );

        // Render
        let max_width = area.width as usize;
        let mut current_width = 0;
        let Rect { mut x, y, .. } = area;
        for (i, g) in self.input.grapheme_indices(true) {
            let grapheme_width = unicode_width::UnicodeWidthStr::width(g);
            current_width += grapheme_width;

            if current_width > max_width + self.scroll {
                break;
            } else if current_width > self.scroll {
                let is_cursor = i == self.cursor_index;
                let is_selected = i >= selection_start && i < selection_end;
                let style = if is_cursor {
                    self.cursor_style
                } else if is_selected {
                    self.selector_style
                } else {
                    Style::new()
                };
                (x, _) = buf.set_stringn(x, y, g, grapheme_width, style);
            }
        }

        if self.cursor_index == self.input.len() {
            buf[(x, y)].set_style(self.cursor_style);
        }

        // Find scroll index
        // let (scroll_index, _) = {
        //     let mut index = 0;
        //     let mut width = 0;
        //     let mut graphemes = self.input[..self.cursor_index].graphemes(true);
        //     while let Some(g) = graphemes.next() {
        //         if width >= self.scroll {
        //             break;
        //         }

        //         index += g.len();
        //         width += unicode_width::UnicodeWidthStr::width(g);
        //     }
        //     (index, width)
        // };

        // // Render
        // buf.set_stringn(
        //     area.x,
        //     area.y,
        //     &self.input[scroll_index..],
        //     area.width as usize,
        //     Style::new(),
        // );

        // // Update scroll value
        // // todo: fix scroll when left-most char has width > 1
        // let line_width = area.width as usize;
        // if scroll_column > self.scroll {
        //     let width_diff = scroll_column - self.scroll;
        //     let line_width = line_width.saturating_sub(1);
        //     if width_diff > line_width {
        //         self.scroll += width_diff - line_width;
        //     }
        // } else if self.scroll > scroll_column {
        //     let width_diff = self.scroll - scroll_column;
        //     self.scroll -= width_diff;
        // }

        //------------------------

        // let mut graphemes = self.input.grapheme_indices(true);

        // loop {
        //     let Some((i, g)) = graphemes.next() else {
        //         if self.cursor_index == input_len {
        //             self.cursor_column = total_width;
        //             self.spans.push(Span::styled(" ", cursor_style));
        //         }
        //         break;
        //     };

        //     let is_cursor = i == self.cursor_index;
        //     let is_selected = i >= selection_start && i < selection_end;

        //     let style = if is_cursor {
        //         self.cursor_column = total_width;
        //         cursor_style
        //     } else if is_selected {
        //         selector_style
        //     } else {
        //         Style::new()
        //     };

        //     let span = Span::styled(g.to_string(), style);
        //     total_width += span.width();
        //     self.spans.push(span);
        // }

        // if self.input.is_empty() {
        //     self.spans
        //         .push(Span::styled(self.placeholder, self.placeholder_style));
        // }

        // // todo: fix scroll when left-most char has width > 1
        // let line_width = area.width as usize;
        // if self.cursor_column > self.scroll {
        //     let width_diff = self.cursor_column - self.scroll;
        //     let line_width = line_width.saturating_sub(1);
        //     if width_diff > line_width {
        //         self.scroll += width_diff - line_width;
        //     }
        // } else if self.scroll > self.cursor_column {
        //     let width_diff = self.scroll - self.cursor_column;
        //     self.scroll -= width_diff;
        // }

        // let mut skip_width = 0;
        // let mut input_width = 0;
        // let mut span_area = Rect { height: 1, ..area };

        // for span in self.spans.iter() {
        //     let span_width = span.width();
        //     skip_width += span_width;
        //     if skip_width > self.scroll && input_width < line_width {
        //         input_width += span_width;
        //         span_area.width = span_width as u16;
        //         (&span).render(span_area, buf);
        //         span_area.x += span_width as u16;
        //     }
        // }
    }
}

fn calculate_scroll(
    total_lines: usize,
    viewport_height: u16,
    selected: usize,
    offset: usize,
    margin_top: usize,
    margin_bottom: usize,
    padding_bottom: usize,
) -> usize {
    let viewport_height = viewport_height as usize;

    let max_offset = total_lines
        .saturating_sub(viewport_height)
        .saturating_add(padding_bottom);

    let available = viewport_height.saturating_sub(1);
    let margin_top = margin_top.min(available);
    let margin_bottom = margin_bottom.min(available - margin_top);

    let top_boundary = offset + margin_top;
    let bottom_boundary = offset + viewport_height.saturating_sub(margin_bottom + 1);

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
