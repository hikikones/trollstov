use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{HorizontalAlignment, Rect},
    style::Color,
    widgets::{Block, Padding, Widget},
};
use shared::symbols;
use widgets::{
    AnsiTag, AnsiViewMode, AnsiViewer, AnsiWriter, RectExt, ScrollMove, Shortcut, Shortcuts,
    TextInput,
};

use crate::{
    app::{Action, AppInput, AppRender},
    database::{CardId, Database},
    pages::{CardsParam, Route},
    settings::Colors,
};

// TODO: Render a help text as markup when search comes up empty.
// Or just add a help shortcut that shows how to search.

pub struct SearchPage {
    state: State,
    search: TextInput,
    results: Vec<CardId>,
    index: usize,
    query: String,
    writer: AnsiWriter,
    viewer: AnsiViewer,
    is_empty: bool,
}

enum State {
    Search,
    Browse,
}

impl SearchPage {
    pub const fn new() -> Self {
        Self {
            state: State::Search,
            search: TextInput::new().with_placeholder("Search..."),
            results: Vec::new(),
            index: 0,
            query: String::new(),
            writer: AnsiWriter::new(),
            viewer: AnsiViewer::new().with_padding(Padding::horizontal(1)),
            is_empty: false,
        }
    }

    pub fn on_enter(&mut self, db: &Database) {
        self.state = State::Search;
        self.is_empty = db.is_cards_empty();
        self.refresh(db);
    }

    pub fn on_render(&mut self, render: AppRender, colors: &Colors, shortcuts: &mut Shortcuts) {
        let (mut area, buf) = render.area_and_buffer();

        if self.is_empty {
            widgets::print_ascii(
                area,
                buf,
                "No cards to search for",
                colors.neutral,
                Some(widgets::Alignment::Center),
            );
            return;
        }

        let (search_color, result_color) = {
            match self.state {
                State::Search => {
                    shortcuts.push(Shortcut::new("Confirm", symbols::ENTER));
                    (colors.secondary, colors.neutral)
                }
                State::Browse => {
                    if self.current_card().is_some() {
                        shortcuts.extend([Shortcut::new("Edit", "e"), Shortcut::new("Goto", "g")]);
                    }
                    shortcuts.push(Shortcut::new("Search", "s"));
                    (colors.neutral, colors.secondary)
                }
            }
        };

        self.render_search(&mut area, buf, search_color, colors);
        self.render_result(area, buf, result_color, colors.neutral);
    }

    fn render_search(
        &mut self,
        area: &mut Rect,
        buf: &mut Buffer,
        border_color: Color,
        colors: &Colors,
    ) {
        let inner = {
            let search_area = Rect { height: 3, ..*area };
            let block = Block::bordered()
                .title(" Search ")
                .title_alignment(HorizontalAlignment::Center)
                .title_style(colors.neutral)
                .border_style(border_color)
                .padding(Padding::horizontal(1));
            let inner = block.inner(search_area);
            block.render(search_area, buf);
            area.shrink_down(search_area.height);
            inner
        };

        self.search
            .set_colors(colors.text_input())
            .set_enabled(matches!(self.state, State::Search))
            .render(inner, buf);
    }

    fn render_result(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        border_color: Color,
        neutral_color: Color,
    ) {
        let inner = {
            let block = Block::bordered().border_style(border_color);
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        };

        utils::format_int2(self.index + 1, self.results.len(), |i, len| {
            widgets::print_asciis(
                area,
                buf,
                [" ", i, " / ", len, " "],
                neutral_color,
                Some(widgets::Alignment::CenterHorizontal),
            );
        });

        let (text_alignment, view_mode) = match self.current_card() {
            Some(_) => (
                HorizontalAlignment::Left,
                AnsiViewMode::Page {
                    scrollbar: false,
                    scrollbar_margin: 1,
                },
            ),
            None => (
                HorizontalAlignment::Center,
                AnsiViewMode::Text {
                    center_vertical: true,
                },
            ),
        };

        self.viewer
            .set_text_alignment(text_alignment)
            .set_view_mode(view_mode)
            .render(inner, buf, self.writer.as_str());
    }

    pub fn on_input(&mut self, input: AppInput, db: &Database, colors: &Colors) -> Action {
        if self.is_empty {
            return Action::None;
        }

        let (key, modifiers) = input.key_pressed_and_modifiers();

        match self.state {
            State::Search => match key {
                KeyCode::Enter => {
                    let input = self.search.as_str_trim();
                    if input.is_empty() {
                        return Action::None;
                    }

                    self.index = 0;
                    self.results.clear();
                    self.writer.clear();
                    self.query.clear();
                    self.query.push_str(input);

                    if let Err(err) = db.search(input, |id| self.results.push(id)) {
                        self.writer
                            .push_fmt(format_args!("{}{err}", AnsiTag::FgRed));
                        return Action::Render;
                    }

                    if input.chars().count() < 3 {
                        self.writer.push_fmt(format_args!(
                            "{}Search query must be at least 3 characters",
                            AnsiTag::FgYellow
                        ));
                        return Action::Render;
                    }

                    if self.results.is_empty() {
                        self.writer.push_fmt(format_args!(
                            "{}No cards found from query\n{}'{input}'",
                            AnsiTag::from_color_fg(colors.neutral),
                            AnsiTag::Italic
                        ));
                        return Action::Render;
                    }

                    self.highlight(self.results[0], db);
                    self.viewer.scroll(ScrollMove::Start);
                    self.state = State::Browse;
                    return Action::Render;
                }
                KeyCode::Down => {
                    if !self.results.is_empty() {
                        self.state = State::Browse;
                        return Action::Render;
                    }
                }
                KeyCode::Up => {}
                _ => {
                    if self.search.input(key, modifiers) {
                        return Action::Render;
                    }
                }
            },
            State::Browse => match key {
                KeyCode::Up => {
                    if self.viewer.current_scroll() == 0 {
                        self.state = State::Search;
                        return Action::Render;
                    } else if self.viewer.scroll(ScrollMove::Up) {
                        return Action::Render;
                    }
                }
                KeyCode::Right => {
                    if self.results.len() > 1 {
                        self.index = (self.index + 1) % self.results.len();
                        self.highlight(self.current_card().unwrap(), db);
                        self.viewer.scroll(ScrollMove::Start);
                        return Action::Render;
                    }
                }
                KeyCode::Left => {
                    if self.results.len() > 1 {
                        if self.index == 0 {
                            self.index = self.results.len() - 1;
                        } else {
                            self.index -= 1;
                        }
                        self.highlight(self.current_card().unwrap(), db);
                        self.viewer.scroll(ScrollMove::Start);
                        return Action::Render;
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(id) = self.current_card() {
                        return Action::Route(Route::Editor(Some(id)));
                    }
                }
                KeyCode::Char('g') => {
                    if let Some(id) = self.current_card() {
                        return Action::Route(Route::Cards(Some(CardsParam::Card(id))));
                    }
                }
                KeyCode::Char('s') => {
                    self.state = State::Search;
                    return Action::Render;
                }
                _ => {
                    if self.viewer.input(key) {
                        return Action::Render;
                    }
                }
            },
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn current_card(&self) -> Option<CardId> {
        self.results.get(self.index).copied()
    }

    fn highlight(&mut self, id: CardId, db: &Database) {
        db.search_highlight(id, &self.query, |content| {
            self.writer.clear();
            self.writer.push_str(content);
        })
        .unwrap();
    }

    fn refresh(&mut self, db: &Database) {
        if self.is_empty || self.query.is_empty() {
            self.results.clear();
            self.index = 0;
            return;
        }

        self.results.clear();
        let _ = db.search(self.query.as_str(), |id| self.results.push(id));
        self.index = self.index.min(self.results.len().saturating_sub(1));

        if let Some(id) = self.current_card() {
            self.highlight(id, db);
        }
    }
}
