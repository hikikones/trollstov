use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, TrackId},
    utils,
    widgets::TextInput,
};

pub struct SearchPage {
    index: usize,
    scroll: usize,
    search_input: TextInput,
    search_results: Vec<(TrackId, u16)>,
    matcher: Matcher,
    is_dirty: bool,
    events: EventSender,
    buffer: String,
}

impl SearchPage {
    pub fn new(colors: &Colors, events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            search_input: TextInput::new(colors.on_accent, colors.accent, colors.neutral)
                .with_placeholder("search..."),
            search_results: Vec::new(),
            matcher: Matcher::new(),
            is_dirty: false,
            events,
            buffer: String::new(),
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        if jb.is_empty() {
            utils::print_ascii(
                area,
                buf,
                "No tracks to search for",
                Style::new().fg(colors.neutral),
                Alignment::Center,
            );
            return;
        }

        self.search_input
            .render(area.centered_horizontally(Constraint::Percentage(60)), buf);

        // Update search results
        if self.is_dirty {
            self.is_dirty = false;
            self.index = 0;
            self.scroll = 0;
            self.search_results.clear();

            if !self.search_input.as_str().trim().is_empty() {
                self.matcher.update(self.search_input.as_str());
                self.search_results
                    .extend(jb.iter().filter_map(|(id, track)| {
                        self.buffer.extend([
                            track.artist(),
                            " ",
                            track.album(),
                            " ",
                            track.title(),
                        ]);
                        let score = self.matcher.score(&self.buffer);
                        self.buffer.clear();
                        score.map(|score| (id, score))
                    }));
                self.search_results
                    .sort_by_key(|(_, score)| std::cmp::Reverse(*score));
            }
        }

        // Render search results
        let area = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(2),
            ..area
        };

        self.scroll = utils::calculate_scroll(self.index, area.height, self.scroll);
        let mut line_area = Rect { height: 1, ..area };
        let current = jb.current_track();

        self.search_results
            .iter()
            .copied()
            .filter_map(|(id, _)| jb.get(id).map(|track| (id, track)))
            .enumerate()
            .skip(self.scroll)
            .take(area.height as usize)
            .for_each(|(i, (id, track))| {
                self.buffer
                    .extend([track.artist(), " ", track.album(), " ", track.title()]);

                let mut style = Style::new();
                if self.index == i {
                    style.bg = Some(colors.accent);
                    style.fg = Some(colors.on_accent);
                }
                if current == Some(id) {
                    style.add_modifier.insert(Modifier::BOLD);
                }

                Line::styled(&self.buffer, style).render(line_area, buf);

                self.buffer.clear();
                line_area.y += 1;
            });
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index =
                    usize::min(self.index + 1, self.search_results.len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            KeyCode::Enter => {
                if let Some((id, _)) = self.search_results.get(self.index).copied() {
                    jb.play(id);
                }
            }
            _ => {
                let hash_old = seahash::hash(self.search_input.as_str().trim().as_bytes());
                if self.search_input.input(key, modifiers) {
                    let hash_new = seahash::hash(self.search_input.as_str().trim().as_bytes());
                    self.is_dirty = hash_old != hash_new;
                    self.events.send(AppEvent::Render);
                }
            }
        }
    }

    pub fn on_exit(&self) {}
}

pub struct Matcher {
    matcher: nucleo_matcher::Matcher,
    atom: nucleo_matcher::pattern::Atom,
    buffer: Vec<char>,
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            matcher: nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            atom: Self::create_atom(""),
            buffer: Vec::new(),
        }
    }

    pub fn update(&mut self, needle: &str) {
        self.atom = Self::create_atom(needle);
    }

    pub fn score(&mut self, haystack: &str) -> Option<u16> {
        self.atom.score(
            nucleo_matcher::Utf32Str::new(haystack, &mut self.buffer),
            &mut self.matcher,
        )
    }

    fn create_atom(needle: &str) -> nucleo_matcher::pattern::Atom {
        nucleo_matcher::pattern::Atom::new(
            needle,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Smart,
            nucleo_matcher::pattern::AtomKind::Fuzzy,
            true,
        )
    }
}
