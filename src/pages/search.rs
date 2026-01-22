use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    editor::TextInput,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, TrackId},
};

pub struct SearchPage {
    index: usize,
    scroll: usize,
    input: TextInput,
    tracks: Vec<(TrackId, u32)>,
    matcher: Matcher,
    is_dirty: bool,
    events: EventSender,
    line_buffer: String,
}

impl SearchPage {
    pub fn new(events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            input: TextInput::new().with_placeholder("search..."),
            tracks: Vec::new(),
            matcher: Matcher::new(),
            is_dirty: false,
            events,
            line_buffer: String::new(),
        }
    }

    pub fn on_enter(&mut self) {
        // todo
        // for _ in 0..100 {
        //     self.tracks.push(TrackId::default());
        // }
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        colors: &Colors,
        menu: &mut Line,
    ) {
        self.input.render(
            area.centered_horizontally(Constraint::Percentage(60)),
            buf,
            colors,
        );

        if self.is_dirty {
            self.is_dirty = false;
            self.matcher.update(self.input.as_str());
            self.tracks.clear();
            self.tracks.extend(jb.iter().filter_map(|(id, track)| {
                self.line_buffer
                    .extend([track.artist(), " ", track.album(), " ", track.title()]);
                let score = self.matcher.score(&self.line_buffer);
                self.line_buffer.clear();
                score.map(|score| (id, score))
            }));
            self.tracks
                .sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        }

        let area = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(2),
            ..area
        };

        let height = area.height as usize;
        if self.index > self.scroll {
            let height_diff = self.index - self.scroll;
            let height = height.saturating_sub(1);
            if height_diff > height {
                self.scroll += height_diff - height;
            }
        } else if self.scroll > self.index {
            let height_diff = self.scroll - self.index;
            self.scroll -= height_diff;
        }

        let mut buffer = itoa::Buffer::new();
        let mut line_area = Rect { height: 1, ..area };

        self.tracks
            .iter()
            .copied()
            .filter_map(|(id, _)| jb.get(id))
            .enumerate()
            .skip(self.scroll)
            .take(height)
            .for_each(|(i, track)| {
                self.line_buffer.extend([
                    buffer.format(i + 1),
                    " ",
                    track.artist(),
                    " ",
                    track.album(),
                    " ",
                    track.title(),
                ]);

                let style = if self.index == i {
                    Style::new().bg(colors.accent).fg(colors.on_accent)
                } else {
                    Style::new()
                };

                Line::styled(&self.line_buffer, style).render(line_area, buf);

                self.line_buffer.clear();
                line_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, self.tracks.len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                // todo
                // let id = jb.get_key_from_index(self.index).unwrap();
                // jb.play(id);
            }
            _ => {
                if self.input.input(key, modifiers) {
                    self.is_dirty = true;
                    self.events.send(AppEvent::Render);
                }
            }
        }
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}

pub struct Matcher {
    matcher: nucleo_matcher::Matcher,
    pattern: nucleo_matcher::pattern::Pattern,
    buffer: Vec<char>,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            matcher: nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            pattern: nucleo_matcher::pattern::Pattern::new(
                "",
                nucleo_matcher::pattern::CaseMatching::Smart,
                nucleo_matcher::pattern::Normalization::Smart,
                nucleo_matcher::pattern::AtomKind::Fuzzy,
            ),
            buffer: Vec::new(),
        }
    }

    pub fn update(&mut self, pattern: &str) {
        self.pattern.reparse(
            pattern,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Smart,
        );
    }

    pub fn score(&mut self, haystack: &str) -> Option<u32> {
        self.pattern.score(
            nucleo_matcher::Utf32Str::new(haystack, &mut self.buffer),
            &mut self.matcher,
        )
    }
}
