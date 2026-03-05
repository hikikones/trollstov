use jukebox::{AudioRating, Database, Jukebox, TrackId, TrackSort};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    settings::Colors,
    symbols,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, utils},
};

pub struct TracksPage {
    list: List,
    keep_on_sort: bool,
}

impl TracksPage {
    pub const fn new() -> Self {
        Self {
            list: List::new(),
            keep_on_sort: false,
        }
    }

    pub const fn set_keep_on_sort(&mut self, value: bool) {
        self.keep_on_sort = value;
    }

    fn is_index_current(&self, db: &Database, jb: &Jukebox) -> bool {
        let current = jb.current_track_id();
        current.is_none() || current == db.get_id_from_index(self.list.index())
    }

    pub fn on_enter(&mut self, id: Option<TrackId>, db: &Database) {
        if let Some(id) = id
            && let Some(index) = db.get_index_from_id(id)
        {
            self.list.move_index(ListMove::Custom(index), false);
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
            utils::print_ascii(
                area,
                buf,
                "No tracks to be found",
                Style::new().fg(colors.neutral),
                Some(utils::Alignment::Center),
            );
            return;
        }

        // Bordered block for tracks table
        let block = Block::bordered().padding(Padding::horizontal(1));
        let tracks_area = block.inner(area);
        block.render(area, buf);

        // Title for bordered tracks table
        jukebox::utils::format_int(db.len(), |len| {
            utils::print_asciis(
                Rect {
                    y: area.y,
                    height: 1,
                    ..tracks_area
                },
                buf,
                [" All tracks (", len, ") "],
                Style::new(),
                Some(utils::Alignment::CenterHorizontal),
            );
        });

        // Render tracks table
        self.render_tracks(tracks_area, buf, db, jb, colors);

        // Shortcuts
        shortcuts.extend([
            Shortcut::new("Play", symbols::ENTER),
            Shortcut::new("Add to queue", "q"),
            Shortcut::new("Play next", "n"),
            Shortcut::new("Rating", "0-5"),
            Shortcut::new("Sort", symbols::shift!("s")),
        ]);

        // Add goto when currently playing track is not selected
        if !self.is_index_current(db, jb) {
            shortcuts.push(Shortcut::new("Goto current", "g"));
        }
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
                if let Some(id) = db.get_id_from_index(self.list.index()) {
                    jb.play_id(id, db);
                }
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    for i in self.list.selection() {
                        if let Some(id) = db.get_id_from_index(i) {
                            db.write_rating(id, rating);
                        }
                    }
                }
                'q' => {
                    for i in self.list.selection() {
                        if let Some(id) = db.get_id_from_index(i) {
                            jb.enqueue(id);
                        }
                    }
                }
                'n' => {
                    for i in self.list.selection().rev() {
                        if let Some(id) = db.get_id_from_index(i) {
                            jb.enqueue_next(id);
                        }
                    }
                }
                's' | 'S' => {
                    let id = db.get_id_from_index(self.list.index());

                    if c == 's' {
                        db.sort(db.get_sort().next());
                    } else {
                        db.sort(db.get_sort().prev());
                    }

                    if self.keep_on_sort
                        && let Some(id) = id
                        && let Some(i) = db.get_index_from_id(id)
                    {
                        self.list.move_index(ListMove::Custom(i), false);
                    }
                    return Action::Render;
                }
                'g' | 'G' => {
                    if !self.is_index_current(db, jb)
                        && let Some(id) = jb.current_track_id()
                        && let Some(index) = db.get_index_from_id(id)
                    {
                        let shift = modifiers.contains(KeyModifiers::SHIFT);
                        self.list.move_index(ListMove::Custom(index), shift);
                        return Action::Render;
                    }
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
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn render_tracks(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        colors: &Colors,
    ) {
        if area.is_empty() {
            return;
        }

        let spacing = 2;
        let shrink_point = (0.20 * area.width as f32).floor() as u16;
        let time_width = shrink_point.min(5 + spacing);
        let rating_width = shrink_point.min(7);
        let scrollbar_width = if db.len() > area.height as usize {
            1
        } else {
            0
        };
        let remaining_width = area
            .width
            .saturating_sub(time_width + rating_width + scrollbar_width);
        let title_width = (0.35 * remaining_width as f32).floor() as u16;
        let album_width = title_width;
        let artist_width = remaining_width - title_width - album_width;

        let header_area = Rect { height: 1, ..area };
        let table_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        };

        // Render the header for the table
        let sort = db.get_sort();
        let mut x = header_area.x;
        for (label, width, spacing) in [
            (
                if sort == TrackSort::TitleAscending {
                    symbols::concat!("Title", symbols::ARROW_HEAD_DOWN)
                } else if sort == TrackSort::TitleDescending {
                    symbols::concat!("Title", symbols::ARROW_HEAD_UP)
                } else {
                    "Title"
                },
                title_width,
                spacing,
            ),
            (
                if sort == TrackSort::ArtistAscending {
                    symbols::concat!("Artist", symbols::ARROW_HEAD_DOWN)
                } else if sort == TrackSort::ArtistDescending {
                    symbols::concat!("Artist", symbols::ARROW_HEAD_UP)
                } else {
                    "Artist"
                },
                artist_width,
                spacing,
            ),
            (
                if sort == TrackSort::AlbumAscending {
                    symbols::concat!("Album", symbols::ARROW_HEAD_DOWN)
                } else if sort == TrackSort::AlbumDescending {
                    symbols::concat!("Album", symbols::ARROW_HEAD_UP)
                } else {
                    "Album"
                },
                album_width,
                spacing,
            ),
            (
                if sort == TrackSort::TimeAscending {
                    symbols::concat!("Time", symbols::ARROW_HEAD_DOWN)
                } else if sort == TrackSort::TimeDescending {
                    symbols::concat!("Time", symbols::ARROW_HEAD_UP)
                } else {
                    "Time"
                },
                time_width,
                spacing,
            ),
            (
                if sort == TrackSort::RatingAscending {
                    symbols::concat!("Rating", symbols::ARROW_HEAD_DOWN)
                } else if sort == TrackSort::RatingDescending {
                    symbols::concat!("Rating", symbols::ARROW_HEAD_UP)
                } else {
                    "Rating"
                },
                rating_width,
                0,
            ),
        ] {
            buf.set_stringn(
                x,
                header_area.y,
                label,
                width.saturating_sub(spacing) as usize,
                Style::new(),
            );
            x += width;
        }

        // Render the body for the table
        let current = jb.current_track_id();
        self.list.set_colors(colors.neutral, None).render(
            table_area,
            buf,
            db.iter(),
            |line, buf, (id, track), item| {
                let mut style = match item {
                    ListItem::Selected => Style::new().bg(colors.secondary).fg(colors.on_secondary),
                    ListItem::Selection => Style::new().bg(colors.neutral).fg(colors.on_neutral),
                    ListItem::Normal => {
                        if current == Some(id) {
                            Style::new().fg(colors.accent)
                        } else {
                            Style::new()
                        }
                    }
                };

                if jb.is_faulty(id) {
                    style.add_modifier.insert(Modifier::CROSSED_OUT);
                }

                utils::print_text_segments_with_styles(
                    line,
                    buf,
                    [
                        (track.title(), title_width, spacing, style),
                        (track.artist(), artist_width, spacing, style),
                        (track.album(), album_width, spacing, style),
                        (
                            track.duration_display(),
                            time_width,
                            spacing,
                            style.not_crossed_out(),
                        ),
                        (
                            symbols::stars(track.rating()),
                            rating_width,
                            0,
                            style.not_crossed_out(),
                        ),
                    ],
                    Some(style.not_crossed_out()),
                );
            },
        );
    }
}
