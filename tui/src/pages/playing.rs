use jukebox::{AudioRating, Database, Jukebox, QueueIndex};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use ratatui_image::StatefulImage;

use crate::{
    app::{Action, FrontCover, ScreenSize},
    pages::Route,
    settings::Colors,
    symbols,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, utils},
};

pub struct PlayingPage {
    current_queue_index: Option<QueueIndex>,
    list: List,
    view_mode: ViewMode,
}

enum ViewMode {
    Queue,
    Cover,
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            current_queue_index: None,
            list: List::new(),
            view_mode: ViewMode::Queue,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        screen_size: ScreenSize,
        front_cover: &mut FrontCover,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        self.update_scroll_on_new_track(jb);

        match screen_size {
            ScreenSize::Small => {
                match self.view_mode {
                    ViewMode::Queue => {
                        self.render_queue(
                            Rect {
                                height: area.height.saturating_sub(1),
                                ..area
                            },
                            buf,
                            db,
                            jb,
                            colors,
                        );
                    }
                    ViewMode::Cover => {
                        match jb
                            .current_track_id()
                            .and_then(|id| db.get(id).map(|track| track.rating()))
                        {
                            Some(rating) => {
                                let cover_area = self.render_cover(
                                    area.inner(Margin::new(1, 1)),
                                    buf,
                                    front_cover,
                                    colors,
                                );
                                let stars = symbols::stars_split(rating);
                                utils::print_texts_with_styles(
                                    Rect {
                                        y: cover_area.y / 2,
                                        ..area
                                    },
                                    buf,
                                    [
                                        (stars.0, Style::new().fg(colors.accent)),
                                        (stars.1, Style::new().fg(colors.neutral)),
                                    ],
                                    None,
                                    Some(utils::Alignment::CenterHorizontal),
                                );
                            }
                            None => {
                                utils::print_ascii(
                                    area,
                                    buf,
                                    "No track currently playing",
                                    colors.neutral,
                                    Some(utils::Alignment::Center),
                                );
                            }
                        }
                    }
                }

                // Shortcut
                utils::print_asciis_with_styles(
                    Rect {
                        y: area.y + area.height.saturating_sub(1),
                        ..area
                    },
                    buf,
                    [
                        ("v", Style::new().fg(colors.accent)),
                        (" ", Style::new()),
                        ("toggle view", Style::new().fg(colors.neutral)),
                    ],
                    Some(utils::Alignment::CenterHorizontal),
                );
            }
            ScreenSize::Medium | ScreenSize::Large => {
                // Layout
                let [playing_area, _, queue_area] = Layout::horizontal([
                    Constraint::Percentage(40),
                    Constraint::Length(1),
                    Constraint::Min(3),
                ])
                .areas(area);

                // Render track
                match jb
                    .current_track_id()
                    .and_then(|id| db.get(id).map(|track| track.rating()))
                {
                    Some(rating) => {
                        let cover_area = self.render_cover(
                            playing_area.inner(Margin::new(0, 1)),
                            buf,
                            front_cover,
                            colors,
                        );
                        let stars = symbols::stars_split(rating);
                        utils::print_texts_with_styles(
                            Rect {
                                y: cover_area.y + cover_area.height,
                                height: 1,
                                ..cover_area
                            },
                            buf,
                            [
                                (stars.0, Style::new().fg(colors.accent)),
                                (stars.1, Style::new().fg(colors.neutral)),
                            ],
                            None,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    None => {
                        utils::print_ascii(
                            playing_area,
                            buf,
                            "No track currently playing",
                            colors.neutral,
                            Some(utils::Alignment::Center),
                        );
                    }
                }

                // Render play queue
                self.render_queue(queue_area, buf, db, jb, colors);

                // Shortcuts
                shortcuts.extend([
                    Shortcut::new("Play", symbols::ENTER),
                    Shortcut::new("Rating", "0-5"),
                    Shortcut::new("Shuffle", "s"),
                    Shortcut::new("Clear", "c"),
                    Shortcut::new("Goto", "g"),
                ]);
            }
        }
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        db: &mut Database,
        jb: &mut Jukebox,
        screen_size: ScreenSize,
    ) -> Action {
        match key {
            KeyCode::Enter => {
                let index = self.list.index();
                jb.play_index(QueueIndex::from(index), db);
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    if let Some(id) = jb.current_track_id() {
                        let rating = AudioRating::from_char(c).unwrap();
                        db.write_rating(id, rating);
                    }
                }
                'c' => {
                    jb.clear();
                    return Action::Render;
                }
                's' => {
                    jb.shuffle();
                    return Action::Render;
                }
                'g' => {
                    let index = self.list.index();
                    let id = jb.get(QueueIndex::from(index));
                    return Action::Route(Route::Tracks(id));
                }
                'v' => {
                    if screen_size == ScreenSize::Small {
                        let new_mode = match self.view_mode {
                            ViewMode::Queue => ViewMode::Cover,
                            ViewMode::Cover => ViewMode::Queue,
                        };
                        self.view_mode = new_mode;
                        return Action::Render;
                    }
                }
                _ => {}
            },
            _ => {
                if self.list.input(key, KeyModifiers::empty()) {
                    return Action::Render;
                }
            }
        }

        Action::None
    }

    pub fn on_exit(&mut self) {}

    fn update_scroll_on_new_track(&mut self, jb: &Jukebox) {
        let current_queue_index = jb.current_queue_index();
        if self.current_queue_index != current_queue_index {
            self.current_queue_index = current_queue_index;
            if let Some(idx) = current_queue_index {
                self.list.move_index(ListMove::Custom(idx.raw()), false);
            }
        }
    }

    fn render_cover(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        front_cover: &mut FrontCover,
        colors: &Colors,
    ) -> Rect {
        let neutral_style = Style::new().fg(colors.neutral);

        const MAX_COVER_SIZE: u16 = 24;
        let mut img_area = {
            let img_w = area.width.min(MAX_COVER_SIZE * 2);
            let img_h = area.height.min(MAX_COVER_SIZE);
            let img_r = Rect {
                width: img_w,
                height: img_h,
                ..area
            };
            utils::align(img_r, area, utils::Alignment::Center)
        };

        match front_cover {
            FrontCover::None => {
                Block::bordered().style(neutral_style).render(img_area, buf);
                utils::print_ascii(
                    img_area,
                    buf,
                    "NO IMAGE",
                    neutral_style,
                    Some(utils::Alignment::Center),
                );
            }
            FrontCover::Loading => {
                Block::bordered().style(neutral_style).render(img_area, buf);
                utils::print_ascii(
                    img_area,
                    buf,
                    "LOADING",
                    neutral_style,
                    Some(utils::Alignment::Center),
                );
            }
            FrontCover::Ready(image) => {
                let resized_img_area = image.size_for(ratatui_image::Resize::default(), img_area);
                img_area = utils::align(
                    Rect {
                        width: resized_img_area.width,
                        height: resized_img_area.height,
                        ..area
                    },
                    area,
                    utils::Alignment::Center,
                );
                StatefulImage::default().render(img_area, buf, image);
            }
        }

        img_area
    }

    fn render_queue(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        colors: &Colors,
    ) {
        let block = Block::bordered()
            .style(colors.neutral)
            .padding(Padding::horizontal(1));
        let queue_inner_area = block.inner(area);
        block.render(area, buf);

        // Title for bordered play queue
        jukebox::utils::format_int2(jb.history(), jb.queue(), |hlen, qlen| {
            utils::print_asciis(
                Rect {
                    y: area.y,
                    height: 1,
                    ..queue_inner_area
                },
                buf,
                [" History (", hlen, ") / Queue (", qlen, ") "],
                colors.neutral,
                Some(utils::Alignment::CenterHorizontal),
            );
        });

        if jb.is_empty() {
            utils::print_ascii(
                queue_inner_area,
                buf,
                "No tracks in the queue",
                colors.neutral,
                Some(utils::Alignment::Center),
            );
            return;
        }

        let scrolloff = (queue_inner_area.height / 2) as usize;
        self.list
            .set_margins(scrolloff, scrolloff)
            .set_padding(scrolloff);

        let current_queue_index = jb.current_queue_index();
        self.list.set_colors(colors.neutral, None).render(
            queue_inner_area,
            buf,
            jb.iter(),
            |line, buf, (id, qi), item| {
                if let Some(track) = db.get(id) {
                    let mut style = Style::new();

                    if current_queue_index == Some(qi) {
                        style.fg = Some(colors.accent);
                    }
                    if jb.is_faulty(id) {
                        style.add_modifier.insert(Modifier::CROSSED_OUT);
                    }

                    let symbol = if item == ListItem::Selected {
                        symbols::concat!(symbols::SELECTED, " ")
                    } else {
                        ""
                    };

                    utils::print_texts_with_styles(
                        line,
                        buf,
                        [
                            (symbol, style.not_crossed_out()),
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
}
