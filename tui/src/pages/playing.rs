use jukebox::{AudioRating, Jukebox, QueueIndex};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use ratatui_image::StatefulImage;

use crate::{
    app::{Action, Colors, FrontCover, ScreenSize},
    widgets::{List, ListMove, Shortcut, Shortcuts, TextSegment, utils},
};

pub struct PlayingPage {
    current_queue_index: Option<QueueIndex>,
    text: TextSegment,
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
            text: TextSegment::new().with_alignment(Alignment::Center),
            list: List::new(),
            view_mode: ViewMode::Queue,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
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
                            jb,
                            colors,
                        );
                    }
                    ViewMode::Cover => {
                        match jb
                            .current_track_id()
                            .and_then(|id| jb.get(id).map(|track| track.rating()))
                        {
                            Some(rating) => {
                                let cover_area = area.inner(Margin::new(1, 1));
                                self.render_cover(cover_area, buf, front_cover, colors);
                                fill_stars(&mut self.text, rating, colors);
                                self.text.render(area, buf);
                                self.text.clear();
                            }
                            None => {
                                utils::print_ascii(
                                    area,
                                    buf,
                                    "No track currently playing",
                                    colors.neutral,
                                    utils::Alignment::Center,
                                );
                            }
                        }
                    }
                }

                // Shortcut
                self.text.extend([
                    ("v", Style::new().fg(colors.accent)),
                    (" ", Style::new()),
                    ("toggle view", Style::new().fg(colors.neutral)),
                ]);
                self.text.render(
                    Rect {
                        y: area.y + area.height.saturating_sub(1),
                        ..area
                    },
                    buf,
                );
                self.text.clear();
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
                    .and_then(|id| jb.get(id).map(|track| track.rating()))
                {
                    Some(rating) => {
                        let cover_area = self.render_cover(
                            playing_area.inner(Margin::new(0, 1)),
                            buf,
                            front_cover,
                            colors,
                        );
                        fill_stars(&mut self.text, rating, colors);
                        self.text.render(
                            Rect {
                                y: cover_area.y + cover_area.height,
                                height: 1,
                                ..cover_area
                            },
                            buf,
                        );
                        self.text.clear();
                    }
                    None => {
                        utils::print_ascii(
                            playing_area,
                            buf,
                            "No track currently playing",
                            colors.neutral,
                            utils::Alignment::Center,
                        );
                    }
                }

                // Render play queue
                self.render_queue(queue_area, buf, jb, colors);

                // Shortcuts
                shortcuts.extend([
                    Shortcut::new("Play", "↵"),
                    Shortcut::new("Rating", "0-5"),
                    Shortcut::new("Shuffle", "s"),
                    Shortcut::new("Clear", "c"),
                ]);
            }
        }
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        jb: &mut Jukebox,
        screen_size: ScreenSize,
    ) -> Action {
        match key {
            KeyCode::Enter => {
                jb.play_queue_index(self.list.index());
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    if let Some(id) = jb.current_track_id() {
                        let rating = AudioRating::from_char(c).unwrap();
                        jb.set_rating(id, rating);
                    }
                }
                'c' => {
                    jb.queue_clear();
                    return Action::Render;
                }
                's' => {
                    jb.queue_shuffle();
                    return Action::Render;
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
                    utils::Alignment::Center,
                );
            }
            FrontCover::Loading => {
                Block::bordered().style(neutral_style).render(img_area, buf);
                utils::print_ascii(
                    img_area,
                    buf,
                    "LOADING",
                    neutral_style,
                    utils::Alignment::Center,
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

    fn render_queue(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        jukebox::utils::format_int(jb.history_len(), |hlen| {
            self.text
                .extend_as_one([" History (", hlen, ")"], Style::new());
        });
        jukebox::utils::format_int(jb.queue_len(), |qlen| {
            self.text
                .extend_as_one([" / Queue (", qlen, ") "], Style::new());
        });

        let block = Block::bordered()
            .title(self.text.as_str())
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(colors.neutral))
            .padding(Padding::horizontal(1));
        let queue_inner_area = block.inner(area);

        block.render(area, buf);
        self.text.clear();

        if jb.is_queue_empty() {
            utils::print_ascii(
                queue_inner_area,
                buf,
                "No tracks in the queue",
                Style::new().fg(colors.neutral),
                utils::Alignment::Center,
            );
            return;
        }

        let scrolloff = (queue_inner_area.height / 2) as usize;
        self.list
            .set_margins(scrolloff, scrolloff)
            .set_padding(scrolloff);

        let current_queue_index = jb.current_queue_index();
        self.list.render(
            queue_inner_area,
            buf,
            jb.queue_iter(),
            |line, buf, (id, qi), is_index, _| {
                if let Some(track) = jb.get(id) {
                    let mut style = Style::new();
                    if current_queue_index == Some(qi) {
                        style.fg = Some(colors.accent);
                    }
                    let symbol = if is_index { "> " } else { "" };

                    utils::print_line_iter(
                        line,
                        buf,
                        [
                            symbol,
                            track.title(),
                            " ",
                            track.artist(),
                            " ",
                            track.album(),
                        ],
                        style,
                    );
                }
            },
        );
    }
}

fn fill_stars(text: &mut TextSegment, rating: AudioRating, colors: &Colors) {
    let accent = Style::new().fg(colors.accent);
    let neutral = Style::new().fg(colors.neutral);
    match rating {
        AudioRating::None => {
            text.push_str("☆☆☆☆☆", neutral);
        }
        AudioRating::Awful => {
            text.extend([("★", accent), ("☆☆☆☆", neutral)]);
        }
        AudioRating::Bad => {
            text.extend([("★★", accent), ("☆☆☆", neutral)]);
        }
        AudioRating::Ok => {
            text.extend([("★★★", accent), ("☆☆", neutral)]);
        }
        AudioRating::Good => {
            text.extend([("★★★★", accent), ("☆", neutral)]);
        }
        AudioRating::Amazing => {
            text.push_str("★★★★★", accent);
        }
    }
}
