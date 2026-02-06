use std::thread::JoinHandle;

use image::GenericImageView;
use jukebox::{AudioPicture, AudioRating, Jukebox, QueueIndex, TrackId};
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    utils,
    widgets::{List, ListMove, Shortcut, Shortcuts},
};

pub struct PlayingPage {
    current: Option<(TrackId, QueueIndex)>,
    picker: Picker,
    image: FrontCover,
    image_handle: Option<JoinHandle<FrontCover>>,
    play_queue_title: String,
    list: List,
    events: EventSender,
}

impl PlayingPage {
    pub const fn new(picker: Picker, events: EventSender) -> Self {
        Self {
            current: None,
            picker,
            image: FrontCover::None,
            image_handle: None,
            play_queue_title: String::new(),
            list: List::new(),
            events,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_update(&mut self, jb: &Jukebox) -> bool {
        let mut render = false;

        if self.current != jb.current_track() {
            if self.current.map(|(id, _)| id) != jb.current_track_id() {
                // Track has changed, time to update image
                self.update_image(jb);
            }

            if let Some(idx) = jb.current_queue_index() {
                // Update scroll
                self.list.move_index(ListMove::Custom(idx.raw()), false);
            }

            render = true;
            self.current = jb.current_track();
        }

        // Poll thread for finished image loading
        if let Some(handle) = self.image_handle.as_ref() {
            if handle.is_finished() {
                let handle = self.image_handle.take().unwrap();
                self.image = handle.join().unwrap();
                render = true;
            }
        }

        render
    }

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        jb: &Jukebox,
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
        self.render_cover(playing_area, buf, jb, colors);

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

        self.render_queue(queue_area_inner, buf, jb, colors);

        // Shortcuts
        shortcuts.extend([
            Shortcut::new("Play", "↵"),
            Shortcut::new("Rating", "1-5"),
            Shortcut::new("Clear queue", "c"),
        ]);
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Enter => {
                jb.play_queue_index(self.list.index());
            }
            KeyCode::Char(c) => match c {
                '1' | '2' | '3' | '4' | '5' => {
                    let rating = AudioRating::from_char(c).unwrap();
                    let id = jb.get_id_from_queue(self.list.index()).unwrap();
                    jb.set_rating(id, rating);
                }
                'c' => {
                    jb.queue_clear();
                    self.events.send(AppEvent::Render);
                }
                _ => {}
            },
            _ => {
                if self.list.input(key, KeyModifiers::empty()) {
                    self.events.send(AppEvent::Render);
                }
            }
        }
    }

    pub fn on_exit(&mut self) {}

    fn render_cover(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        let neutral_style = Style::new().fg(colors.neutral);

        // Show currently playing, image or not
        match self.current {
            Some((id, _)) => {
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

                match &mut self.image {
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

                let accent_style = Style::new().fg(colors.accent);
                let track = jb.get(id).unwrap();
                match track.rating() {
                    Some(rating) => match rating {
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
                    },
                    None => {
                        Span::styled("☆☆☆☆☆", neutral_style).render(stars_area, buf);
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

        self.list.set_offset((area.height / 2) as usize);

        let current_queue_index = jb.current_queue_index();
        self.list.render(
            area,
            buf,
            jb.queue_iter(),
            |line, buf, (id, qi), is_index, _| {
                let mut style = Style::new();
                if let Some(queue_index) = current_queue_index
                    && queue_index == qi
                {
                    style.fg = Some(colors.accent);
                }
                let symbol = if is_index { "> " } else { "" };

                let track = jb.get(id).unwrap();
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
            },
        );
    }

    fn update_image(&mut self, jb: &Jukebox) {
        match jb.current_track_id() {
            Some(tid) => {
                // Load image in thread and store handle
                self.image = FrontCover::Loading;
                let path = jb.get(tid).unwrap().path().to_path_buf();
                let picker = self.picker.clone();
                let handle = std::thread::spawn(move || {
                    let picture = AudioPicture::read(path).unwrap();
                    match picture.bytes() {
                        Some(bytes) => {
                            const MAX_RES: u32 = 720;
                            let mut dyn_img = image::load_from_memory(bytes).unwrap();
                            let (w, h) = dyn_img.dimensions();
                            if w > MAX_RES || h > MAX_RES {
                                dyn_img = dyn_img.thumbnail(MAX_RES, MAX_RES);
                            }
                            FrontCover::Ready(picker.new_resize_protocol(dyn_img))
                        }
                        None => FrontCover::None,
                    }
                });
                self.image_handle = Some(handle);
            }
            None => {
                // No track currently playing, remove image
                self.image = FrontCover::None;
                self.image_handle = None;
            }
        }
    }
}

enum FrontCover {
    None,
    Loading,
    Ready(StatefulProtocol),
}
