use jukebox::{AudioRating, Jukebox, QueueIndex};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use ratatui_image::StatefulImage;

use crate::{
    app::{Action, Colors, FrontCover},
    widgets::{List, ListMove, Shortcut, Shortcuts, utils},
};

pub struct PlayingPage {
    current_queue_index: Option<QueueIndex>,
    play_queue_title: String,
    list: List,
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            current_queue_index: None,
            play_queue_title: String::new(),
            list: List::new(),
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        front_cover: &mut FrontCover,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        let [playing_area, _, queue_area] = Layout::horizontal([
            Constraint::Percentage(40),
            Constraint::Length(1),
            Constraint::Fill(0),
        ])
        .areas(area);

        // Render track
        self.render_cover(playing_area, buf, jb, front_cover, colors);

        // Render play queue
        jukebox::utils::format_int(jb.history_len(), |hlen| {
            self.play_queue_title.extend([" History (", hlen, ")"]);
        });
        jukebox::utils::format_int(jb.queue_len(), |qlen| {
            self.play_queue_title.extend([" / Queue (", qlen, ") "]);
        });

        let block = Block::bordered()
            .title(self.play_queue_title.as_str())
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(colors.neutral))
            .padding(Padding::horizontal(1));
        let queue_area_inner = block.inner(queue_area);

        block.render(queue_area, buf);
        self.play_queue_title.clear();

        // Update scroll on new track
        if self.current_queue_index != jb.current_queue_index() {
            self.current_queue_index = jb.current_queue_index();
            if let Some(idx) = jb.current_queue_index() {
                self.list.move_index(ListMove::Custom(idx.raw()), false);
            }
        }

        self.render_queue(queue_area_inner, buf, jb, colors);

        // Shortcuts
        shortcuts.extend([
            Shortcut::new("Play", "↵"),
            Shortcut::new("Rating", "0-5"),
            Shortcut::new("Shuffle", "s"),
            Shortcut::new("Clear", "c"),
        ]);
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) -> Action {
        match key {
            KeyCode::Enter => {
                jb.play_queue_index(self.list.index());
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    if let Some(id) = jb.get_id_from_queue(self.list.index()) {
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

    fn render_cover(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
        front_cover: &mut FrontCover,
        colors: &Colors,
    ) {
        let neutral_style = Style::new().fg(colors.neutral);

        // Show currently playing, image or not
        match jb.current_track_id() {
            Some(id) => {
                const MAX_COVER_SIZE: u16 = 20;
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
                        let resized_img_area =
                            image.size_for(ratatui_image::Resize::default(), img_area);
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

                // Show rating as colored stars
                let mut stars_area = utils::align(
                    Rect {
                        y: img_area.y + img_area.height + 1,
                        width: 5,
                        height: 1,
                        ..img_area
                    },
                    img_area,
                    utils::Alignment::CenterHorizontal,
                );

                if let Some(rating) = jb.get(id).map(|track| track.rating()) {
                    let accent_style = Style::new().fg(colors.accent);
                    match rating {
                        AudioRating::None => {
                            Span::styled("☆☆☆☆☆", neutral_style).render(stars_area, buf);
                        }
                        AudioRating::Awful => {
                            Span::styled("★", accent_style).render(stars_area, buf);
                            stars_area.x += 1;
                            Span::styled("☆☆☆☆", neutral_style).render(stars_area, buf);
                        }
                        AudioRating::Bad => {
                            Span::styled("★★", accent_style).render(stars_area, buf);
                            stars_area.x += 2;
                            Span::styled("☆☆☆", neutral_style).render(stars_area, buf);
                        }
                        AudioRating::Ok => {
                            Span::styled("★★★", accent_style).render(stars_area, buf);
                            stars_area.x += 3;
                            Span::styled("☆☆", neutral_style).render(stars_area, buf);
                        }
                        AudioRating::Good => {
                            Span::styled("★★★★", accent_style).render(stars_area, buf);
                            stars_area.x += 4;
                            Span::styled("☆", neutral_style).render(stars_area, buf);
                        }
                        AudioRating::Amazing => {
                            Span::styled("★★★★★", accent_style).render(stars_area, buf);
                        }
                    }
                }
            }
            None => {
                utils::print_ascii(
                    area,
                    buf,
                    "No track currently playing",
                    neutral_style,
                    utils::Alignment::Center,
                );
            }
        }
    }

    fn render_queue(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        if jb.is_queue_empty() {
            utils::print_ascii(
                area,
                buf,
                "No tracks in the queue",
                Style::new().fg(colors.neutral),
                utils::Alignment::Center,
            );
            return;
        }

        let scrolloff = (area.height / 2) as usize;
        self.list
            .set_margins(scrolloff, scrolloff)
            .set_padding(scrolloff);

        let current_queue_index = jb.current_queue_index();
        self.list.render(
            area,
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
