use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Padding, Widget},
};
use shared::symbols;
use widgets::{Shortcut, Shortcuts, Table, TableItem, TableLayout, TableMove};

use crate::{
    app::Action,
    database::{AudioRating, Database, Track, TrackId, TrackSort},
    jukebox::Jukebox,
    settings::Colors,
};

const N: usize = 5;

pub struct TracksPage {
    table: Table<N>,
    reverse_sort: bool,
    keep_on_sort: bool,
}

impl TracksPage {
    pub const fn new() -> Self {
        Self {
            table: Table::new(TableLayout::new(
                [
                    Constraint::Ratio(4, 10),
                    Constraint::Ratio(2, 10),
                    Constraint::Ratio(4, 10),
                    Constraint::Length(5),
                    Constraint::Length(7),
                ],
                2,
            ))
            .with_padding(Padding::horizontal(1))
            .with_scrollbar(),
            reverse_sort: false,
            keep_on_sort: false,
        }
    }

    pub const fn set_keep_on_sort(&mut self, value: bool) {
        self.keep_on_sort = value;
    }

    pub fn on_enter(&mut self, id: Option<TrackId>, db: &Database) {
        if let Some(id) = id
            && let Some(index) = db.get_index_from_id(id)
        {
            self.table.set_index(index).set_selector(None);
        };
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        if db.is_empty() {
            widgets::print_ascii(
                area,
                buf,
                "No tracks to be found",
                Style::new().fg(colors.neutral),
                Some(widgets::Alignment::Center),
            );
            return;
        }

        let inner = {
            let block = Block::bordered().border_style(colors.secondary);
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        };

        utils::format_int(db.len(), |len| {
            widgets::print_asciis(
                area,
                buf,
                [" All Tracks (", len, ") "],
                Color::Reset,
                Some(widgets::Alignment::CenterHorizontal),
            );
        });

        let sort = db.get_sort();
        let reverse = self.reverse_sort;
        let current = jb.current_track_id();

        self.table.set_colors(colors.table()).render(
            inner,
            buf,
            db.iter(),
            |_header, buf, areas| {
                for (name, symbol, area) in named_headers(areas, sort, reverse) {
                    let (x, y) =
                        buf.set_stringn(area.x, area.y, name, area.width as usize, Style::new());
                    buf.set_string(x, y, symbol, Style::new());
                }
            },
            |row, buf, areas, (id, track), item| {
                let mut style = match item {
                    TableItem::Selected => Style::new().fg(colors.primary).reversed(),
                    TableItem::Selection => Style::new().fg(colors.neutral).reversed(),
                    TableItem::Normal => Style::new(),
                };

                if current == Some(id) {
                    style.add_modifier.insert(Modifier::BOLD);
                }

                if jb.is_faulty(id) {
                    style.add_modifier.insert(Modifier::CROSSED_OUT);
                }

                buf.set_style(row, style);

                for (name, area) in named_rows(areas, track) {
                    buf.set_stringn(area.x, area.y, name, area.width as usize, Style::new());
                }
            },
        );

        shortcuts.extend([
            Shortcut::new("Play", symbols::ENTER),
            Shortcut::new("Add to queue", "q"),
            Shortcut::new("Play next", "n"),
            Shortcut::new("Rating", "0-5"),
            Shortcut::new("Sort", symbols::shift!("s")),
        ]);
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        db: &mut Database,
        jb: &mut Jukebox,
    ) -> Action {
        match key {
            KeyCode::Enter => {
                if let Some(id) = db.get_id_from_index(self.table.index()) {
                    jb.play_id(id, db);
                }
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    for i in self.table.selection_inclusive() {
                        if let Some(id) = db.get_id_from_index(i) {
                            db.write_rating(id, rating);
                        }
                    }
                }
                'q' => {
                    let ids = self
                        .table
                        .selection_inclusive()
                        .filter_map(|i| db.get_id_from_index(i));
                    jb.extend(ids);
                }
                'n' => {
                    for i in self.table.selection_inclusive().rev() {
                        if let Some(id) = db.get_id_from_index(i) {
                            jb.enqueue_next(id);
                        }
                    }
                }
                's' | 'S' => {
                    let id = db.get_id_from_index(self.table.index());

                    if c == 's' {
                        db.sort(db.get_sort().next(), self.reverse_sort);
                    } else {
                        self.reverse_sort = !self.reverse_sort;
                        db.sort(db.get_sort(), self.reverse_sort);
                    }

                    if self.keep_on_sort
                        && let Some(id) = id
                        && let Some(i) = db.get_index_from_id(id)
                    {
                        self.table.move_index(TableMove::Custom(i), false);
                    }
                    return Action::Render;
                }
                _ => {
                    if self.table.input(key, modifiers) {
                        return Action::Render;
                    }
                }
            },
            _ => {
                if self.table.input(key, modifiers) {
                    return Action::Render;
                }
            }
        }

        Action::None
    }

    pub fn on_exit(&self) {}
}

const fn named_areas(areas: [Rect; N]) -> [(TrackSort, Rect); N] {
    let [title, artist, album, time, rating] = areas;
    [
        (TrackSort::Title, title),
        (TrackSort::Artist, artist),
        (TrackSort::Album, album),
        (TrackSort::Time, time),
        (TrackSort::Rating, rating),
    ]
}

fn named_headers<'a>(
    areas: [Rect; N],
    current_sort: TrackSort,
    reverse_sort: bool,
) -> impl Iterator<Item = (&'a str, &'a str, Rect)> {
    fn sort_symbol<'a>(current_sort: TrackSort, sort: TrackSort, reverse: bool) -> &'a str {
        if current_sort == sort {
            if reverse {
                symbols::ARROW_HEAD_UP
            } else {
                symbols::ARROW_HEAD_DOWN
            }
        } else {
            ""
        }
    }

    named_areas(areas).into_iter().map(move |(sort, area)| {
        (
            sort.as_str(),
            sort_symbol(current_sort, sort, reverse_sort),
            area,
        )
    })
}

fn named_rows<'a>(areas: [Rect; N], track: &'a Track) -> impl Iterator<Item = (&'a str, Rect)> {
    named_areas(areas)
        .into_iter()
        .map(|(sort, area)| (track.field_str(sort), area))
}
