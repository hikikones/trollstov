use jukebox::{Jukebox, TrackId};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    pages::Route,
    settings::Colors,
    symbols,
    widgets::{List, ListItem, Shortcut, Shortcuts, TextInput, TextInputStyles, utils},
};

pub struct SearchPage {
    state: State,
    search_input: TextInput,
    search_results: Vec<(TrackId, u16)>,
    list: List,
    is_dirty: bool,
}

enum State {
    Search,
    Browse,
}

impl SearchPage {
    pub const fn new() -> Self {
        Self {
            state: State::Search,
            search_input: TextInput::new()
                .with_placeholder("Search...")
                .with_margins(2, 2),
            search_results: Vec::new(),
            list: List::new(),
            is_dirty: false,
        }
    }

    pub const fn set_search(&mut self) {
        self.state = State::Search;
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &mut Jukebox,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        if jb.is_empty() {
            utils::print_ascii(
                area,
                buf,
                "No tracks to search for",
                colors.neutral,
                Some(utils::Alignment::Center),
            );
            return;
        }

        // Determine colors and shortcuts for search input and results
        let (block_style, border_style) = {
            let neutral = Style::new().fg(colors.neutral);
            match self.state {
                State::Search => {
                    self.search_input.set_styles(TextInputStyles {
                        normal: Style::new(),
                        cursor: Style::new().bg(colors.accent).fg(colors.on_accent),
                        selector: Style::new().bg(colors.neutral).fg(colors.on_neutral),
                        placeholder: neutral,
                    });
                    shortcuts.extend([
                        Shortcut::new("Browse", symbols::ENTER),
                        Shortcut::new("Select all", symbols::ctrl!("a")),
                    ]);

                    (neutral, neutral)
                }
                State::Browse => {
                    self.search_input.set_styles(TextInputStyles::all(neutral));
                    shortcuts.extend([
                        Shortcut::new("Play", symbols::ENTER),
                        Shortcut::new("Add to queue", "q"),
                        Shortcut::new("Play next", "n"),
                        Shortcut::new("Search", "s"),
                        Shortcut::new("Goto", "g"),
                    ]);

                    (Style::new(), Style::new().fg(colors.accent))
                }
            }
        };

        // Render search input
        let search_line =
            Rect { height: 1, ..area }.centered_horizontally(Constraint::Percentage(64));
        self.search_input.render(search_line, buf);

        // Update search results
        self.update_search_results(jb);

        // Render search results
        let search_results_area = Rect {
            y: area.y + search_line.height + 1,
            height: area.height.saturating_sub(search_line.height + 1),
            ..area
        };
        let search_results_block = Block::bordered()
            .style(block_style)
            .border_style(border_style)
            .padding(Padding::horizontal(1));
        let search_results_inner = search_results_block.inner(search_results_area);
        search_results_block.render(search_results_area, buf);

        // Title for bordered search results
        jukebox::utils::format_int(self.search_results.len(), |len| {
            utils::print_asciis(
                Rect {
                    y: search_results_area.y,
                    height: 1,
                    ..search_results_inner
                },
                buf,
                [" Search results (", len, ") "],
                border_style,
                Some(utils::Alignment::CenterHorizontal),
            );
        });

        let current = jb.current_track_id();
        self.list.set_colors(colors.neutral, None).render(
            search_results_inner,
            buf,
            self.search_results.iter().copied(),
            |line, buf, (id, _), item| {
                if let Some(track) = jb.get(id) {
                    let mut style = Style::new();
                    if matches!(self.state, State::Browse) {
                        match item {
                            ListItem::Selected => {
                                style.bg = Some(colors.accent);
                                style.fg = Some(colors.on_accent);
                            }
                            ListItem::Selection => {
                                style.bg = Some(colors.neutral);
                                style.fg = Some(colors.on_neutral);
                            }
                            ListItem::Normal => {}
                        }
                    }

                    if current == Some(id) {
                        style.add_modifier.insert(Modifier::BOLD);
                    }
                    if jb.is_faulty(id) {
                        style.add_modifier.insert(Modifier::CROSSED_OUT);
                    }

                    utils::print_texts_with_styles(
                        line,
                        buf,
                        [
                            (track.title(), style),
                            (" ", style),
                            (track.artist(), style),
                            (" ", style),
                            (track.album(), style),
                        ],
                        Some(style.not_crossed_out()),
                        None,
                    );
                }
            },
        );
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) -> Action {
        if jb.is_empty() {
            return Action::None;
        }

        match self.state {
            State::Search => match key {
                KeyCode::Enter | KeyCode::Down => {
                    if !self.search_results.is_empty() {
                        self.list.reset();
                        self.state = State::Browse;
                        return Action::Render;
                    }
                }
                KeyCode::Up => {}
                _ => {
                    let hash_old = seahash::hash(self.search_input.as_str().trim().as_bytes());
                    if self.search_input.input(key, modifiers) {
                        let hash_new = seahash::hash(self.search_input.as_str().trim().as_bytes());
                        self.is_dirty = hash_old != hash_new;
                        return Action::Render;
                    }
                }
            },
            State::Browse => match key {
                KeyCode::Enter => {
                    if let Some((id, _)) = self.search_results.get(self.list.index()).copied() {
                        jb.play_track(id);
                    }
                }
                KeyCode::Up => {
                    if self.list.index() == 0 {
                        self.state = State::Search;
                        return Action::Render;
                    } else if self.list.input(key, modifiers) {
                        return Action::Render;
                    }
                }
                KeyCode::Char(c) => match c {
                    'q' => {
                        self.list
                            .selection()
                            .filter_map(|i| self.search_results.get(i).map(|(id, _)| *id))
                            .for_each(|id| {
                                jb.enqueue(id);
                            });
                    }
                    'n' => {
                        self.list
                            .selection()
                            .rev()
                            .filter_map(|i| self.search_results.get(i).map(|(id, _)| *id))
                            .for_each(|id| {
                                jb.enqueue_next(id);
                            });
                    }
                    'g' => {
                        let index = self.list.index();
                        let id = self.search_results.get(index).map(|(id, _)| *id);
                        return Action::Route(Route::Tracks(id));
                    }
                    's' | '/' => {
                        self.state = State::Search;
                        return Action::Render;
                    }
                    _ => {
                        if self.list.input(key, modifiers) {
                            return Action::Render;
                        }
                    }
                },
                _ => {
                    if self.list.input(key, modifiers) {
                        return Action::Render;
                    }
                }
            },
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn update_search_results(&mut self, jb: &mut Jukebox) {
        if !self.is_dirty {
            return;
        }

        self.is_dirty = false;
        self.search_results.clear();

        let keywords = self.search_input.as_str().trim();
        if keywords.is_empty() {
            return;
        }

        self.search_results.extend(jb.search(keywords));
        self.search_results
            .sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    }
}
