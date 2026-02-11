use jukebox::{Jukebox, TrackId};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    utils,
    widgets::{List, ListMove, TextInput},
};

pub struct SearchPage {
    search_input: TextInput,
    search_results: Vec<(TrackId, u16)>,
    matcher: Matcher,
    list: List,
    is_dirty: bool,
    buffer: String,
    events: EventSender,
}

impl SearchPage {
    pub fn new(colors: &Colors, events: EventSender) -> Self {
        Self {
            search_input: TextInput::new().with_placeholder("Search..."),
            search_results: Vec::new(),
            matcher: Matcher::new(),
            list: List::new(),
            is_dirty: false,
            buffer: String::new(),
            events,
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
                utils::Alignment::Center,
            );
            return;
        }

        // Render search input
        let search_input_area = Rect { height: 3, ..area };
        let search_input_block = Block::bordered()
            .title(" Search input ")
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1));
        let search_line = search_input_block.inner(search_input_area);
        search_input_block.render(search_input_area, buf);
        self.search_input.render(search_line, buf);

        // Update search results
        if self.is_dirty {
            self.update_search_results(jb);
        }

        // Render search results
        jukebox::utils::format_int(self.search_results.len(), |len| {
            self.buffer.extend([" Search results (", len, ") "]);
        });

        let search_results_area = Rect {
            y: area.y + search_input_area.height + 1,
            height: area.height.saturating_sub(search_input_area.height + 1),
            ..area
        };
        let search_results_block = Block::bordered()
            .title(self.buffer.as_str())
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1));
        let search_results_inner = search_results_block.inner(search_results_area);

        search_results_block.render(search_results_area, buf);
        self.buffer.clear();

        let current = jb.current_track_id();
        self.list.render(
            search_results_inner,
            buf,
            self.search_results.iter().copied(),
            |line, buf, (id, _), is_index, is_selected| {
                if let Some(track) = jb.get(id) {
                    let mut style = Style::new();
                    if is_index || is_selected {
                        style.bg = Some(colors.accent);
                        style.fg = Some(colors.on_accent);
                    }
                    if current == Some(id) {
                        style.add_modifier.insert(Modifier::BOLD);
                    }

                    utils::print_line_iter(
                        line,
                        buf,
                        [track.artist(), " ", track.album(), " ", track.title()],
                        style,
                    );
                }
            },
        );
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        match key {
            KeyCode::Down => {
                if self.list.move_index(ListMove::Down, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::Up => {
                if self.list.move_index(ListMove::Up, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::PageDown => {
                if self.list.move_index(ListMove::PageDown, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::PageUp => {
                if self.list.move_index(ListMove::PageUp, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::End => {
                if self.list.move_index(ListMove::End, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::Home => {
                if self.list.move_index(ListMove::Start, shift) {
                    self.events.send(AppEvent::Render);
                }
            }
            KeyCode::Enter => {
                if let Some((id, _)) = self.search_results.get(self.list.index()).copied() {
                    jb.play_track(id);
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

    fn update_search_results(&mut self, jb: &Jukebox) {
        self.is_dirty = false;
        self.search_results.clear();

        if !self.search_input.as_str().trim().is_empty() {
            self.matcher.update(self.search_input.as_str());
            self.search_results
                .extend(jb.iter().filter_map(|(id, track)| {
                    self.buffer
                        .extend([track.artist(), " ", track.album(), " ", track.title()]);
                    let score = self.matcher.score(&self.buffer);
                    self.buffer.clear();
                    score.map(|score| (id, score))
                }));
            self.search_results
                .sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        }
    }
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
