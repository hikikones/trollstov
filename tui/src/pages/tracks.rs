use jukebox::{AudioRating, Jukebox, TrackId, TrackSort};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::{Action, Colors},
    widgets::{List, ListMove, Shortcut, Shortcuts, utils},
};

pub struct TracksPage {
    title: String,
    list: List,
}

impl TracksPage {
    pub const fn new() -> Self {
        Self {
            title: String::new(),
            list: List::new(),
        }
    }

    pub fn on_enter(&mut self, id: Option<TrackId>, jb: &Jukebox) {
        if let Some(id) = id
            && let Some(index) = jb.get_index_from_id(id)
        {
            self.list.move_index(ListMove::Custom(index), false);
        };
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        if jb.is_empty() {
            utils::print_ascii(
                area,
                buf,
                "No tracks to be found",
                Style::new().fg(colors.neutral),
                utils::Alignment::Center,
            );
            return;
        }

        // Render tracks table
        jukebox::utils::format_int(jb.len(), |len| {
            self.title.extend([" All tracks (", len, ") "]);
        });

        let block = Block::bordered()
            .title(self.title.as_str())
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1));
        let tracks_area = block.inner(area);

        block.render(area, buf);
        self.title.clear();

        self.render_tracks(tracks_area, buf, jb, colors);

        // Shortcuts
        shortcuts.extend([
            Shortcut::new("Play", "↵"),
            Shortcut::new("Add to queue", "q"),
            Shortcut::new("Play next", "n"),
            Shortcut::new("Rating", "0-5"),
            Shortcut::new("Sort", "(⇧)s"),
        ]);
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) -> Action {
        match key {
            KeyCode::Enter => {
                if let Some(id) = jb.get_id_from_index(self.list.index()) {
                    jb.play_track(id);
                }
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    for i in self.list.selection() {
                        if let Some(id) = jb.get_id_from_index(i) {
                            jb.set_rating(id, rating);
                        }
                    }
                }
                'q' => {
                    for i in self.list.selection() {
                        if let Some(id) = jb.get_id_from_index(i) {
                            jb.enqueue(id);
                        }
                    }
                }
                'n' => {
                    for i in self.list.selection().rev() {
                        if let Some(id) = jb.get_id_from_index(i) {
                            jb.enqueue_next(id);
                        }
                    }
                }
                's' => {
                    let id = jb.get_id_from_index(self.list.index());
                    jb.sort(jb.get_sort().next());
                    if let Some(id) = id
                        && let Some(i) = jb.get_index_from_id(id)
                    {
                        self.list.move_index(ListMove::Custom(i), false);
                    }
                    return Action::Render;
                }
                'S' => {
                    let id = jb.get_id_from_index(self.list.index());
                    jb.sort(jb.get_sort().prev());
                    if let Some(id) = id
                        && let Some(i) = jb.get_index_from_id(id)
                    {
                        self.list.move_index(ListMove::Custom(i), false);
                    }
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
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn render_tracks(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        if area.is_empty() {
            return;
        }

        let spacing = 2;
        let shrink_point = (0.20 * area.width as f32).floor() as u16;
        let time_width = shrink_point.min(5 + spacing);
        let rating_width = shrink_point.min(6);
        let scrollbar_width = if jb.len() > area.height as usize {
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
        let sort = jb.get_sort();
        let mut x = header_area.x;
        for (label, width, spacing) in [
            (
                if sort == TrackSort::TitleAscending {
                    "Title⌄"
                } else if sort == TrackSort::TitleDescending {
                    "Title⌃"
                } else {
                    "Title"
                },
                title_width,
                spacing,
            ),
            (
                if sort == TrackSort::ArtistAscending {
                    "Artist⌄"
                } else if sort == TrackSort::ArtistDescending {
                    "Artist⌃"
                } else {
                    "Artist"
                },
                artist_width,
                spacing,
            ),
            (
                if sort == TrackSort::AlbumAscending {
                    "Album⌄"
                } else if sort == TrackSort::AlbumDescending {
                    "Album⌃"
                } else {
                    "Album"
                },
                album_width,
                spacing,
            ),
            (
                if sort == TrackSort::TimeAscending {
                    "Time⌄"
                } else if sort == TrackSort::TimeDescending {
                    "Time⌃"
                } else {
                    "Time"
                },
                time_width,
                spacing,
            ),
            (
                if sort == TrackSort::RatingAscending {
                    "Rating⌄"
                } else if sort == TrackSort::RatingDescending {
                    "Rating⌃"
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
        self.list.render(
            table_area,
            buf,
            jb.iter(),
            |line, buf, (id, track), is_index, is_selected| {
                let mut style = Style::new();
                if is_index || is_selected {
                    style.bg = Some(colors.accent);
                    style.fg = Some(colors.on_accent);
                }
                if current == Some(id) {
                    style.add_modifier.insert(Modifier::BOLD);
                }

                utils::print_text_segments(
                    line,
                    buf,
                    [
                        (track.title(), title_width, spacing),
                        (track.artist(), artist_width, spacing),
                        (track.album(), album_width, spacing),
                        (track.duration_display(), time_width, spacing),
                        (track.rating_display(), rating_width, 0),
                    ],
                    style,
                );
            },
        );
    }
}
