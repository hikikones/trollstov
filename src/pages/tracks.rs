use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Colors,
    audio::AudioRating,
    events::{AppEvent, EventSender},
    jukebox::{Jukebox, TrackSort},
    utils,
    widgets::{List, Shortcut, Shortcuts},
};

pub struct TracksPage {
    title: String,
    list: List,
    events: EventSender,
}

impl TracksPage {
    pub const fn new(events: EventSender) -> Self {
        Self {
            title: String::new(),
            list: List::new(),
            events,
        }
    }

    pub fn on_enter(&self) {}

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
        let mut buffer = itoa::Buffer::new();
        let len = buffer.format(jb.len());
        self.title.extend([" All tracks (", len, ") "]);

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
            Shortcut::new("Sort", "(⇧)s"),
            Shortcut::new("Add to queue", "q"),
            Shortcut::new("Play next", "n"),
        ]);
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Enter => {
                let id = jb.get_id_from_index(self.list.index()).unwrap();
                jb.play(id);
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    for i in self.list.selection() {
                        let id = jb.get_id_from_index(i).unwrap();
                        jb.set_rating(id, rating);
                    }
                }
                'q' => {
                    for i in self.list.selection() {
                        let id = jb.get_id_from_index(i).unwrap();
                        jb.enqueue_back(id);
                    }
                }
                'n' => {
                    for i in self.list.selection().rev() {
                        let id = jb.get_id_from_index(i).unwrap();
                        jb.enqueue_front(id);
                    }
                }
                's' => {
                    jb.sort(jb.get_sort().next());
                    self.events.send(AppEvent::Render);
                }
                'S' => {
                    jb.sort(jb.get_sort().prev());
                    self.events.send(AppEvent::Render);
                }
                _ => {
                    if self.list.input(key, modifiers) {
                        self.events.send(AppEvent::Render);
                    }
                }
            },
            _ => {
                if self.list.input(key, modifiers) {
                    self.events.send(AppEvent::Render);
                }
            }
        }
    }

    pub fn on_exit(&self) {}

    fn render_tracks(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        let spacing = 2;
        let time_width = 6 + spacing;
        let rating_width = 7;
        let remaining_width = area.width.saturating_sub(time_width + rating_width);
        let info_width = remaining_width / 3;

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
                info_width,
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
                info_width,
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
                info_width,
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
        let current = jb.current_track();
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
                        (track.title(), info_width, spacing),
                        (track.artist(), info_width, spacing),
                        (track.album(), info_width, spacing),
                        (track.duration_display(), time_width, spacing),
                        (track.rating_display(), rating_width, 0),
                    ],
                    style,
                );
            },
        );
    }
}
