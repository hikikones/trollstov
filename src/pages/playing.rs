use std::thread::JoinHandle;

use image::GenericImageView;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::Block,
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use crate::{
    app::Colors,
    audio::AudioPicture,
    jukebox::{Jukebox, TrackId},
    utils,
};

pub struct NowPlayingPage {
    current: Option<TrackId>,
    picker: Picker,
    image: FrontCover,
    image_handle: Option<JoinHandle<FrontCover>>,
}

impl NowPlayingPage {
    pub const fn new(picker: Picker) -> Self {
        Self {
            current: None,
            picker,
            image: FrontCover::None,
            image_handle: None,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_update(&mut self, jb: &Jukebox) {
        if self.current != jb.current_track() {
            self.current = jb.current_track();

            // Track has changed, time to update image
            match jb.current_track() {
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
        } else if let Some(handle) = self.image_handle.take() {
            // Poll thread for finished
            if handle.is_finished() {
                self.image = handle.join().unwrap();
            } else {
                self.image_handle = Some(handle);
            }
        }
    }

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        let neutral_style = Style::new().fg(colors.neutral);

        // Show currently playing, image or not
        match self.current {
            Some(id) => {
                let [left_area, _, right_area] = Layout::horizontal([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Fill(0),
                ])
                .areas(area);

                const MAX_COVER_SIZE: u16 = 15;
                let mut img_area = {
                    let img_w = left_area.width.min(MAX_COVER_SIZE * 2);
                    let img_h = left_area.height.min(MAX_COVER_SIZE);
                    let img_r = Rect {
                        width: img_w,
                        height: img_h,
                        ..left_area
                    };
                    utils::align(img_r, left_area, utils::Alignment::Right)
                };

                match &mut self.image {
                    FrontCover::None => {
                        const NO_IMAGE: &str = "NO IMAGE";
                        Block::bordered().style(neutral_style).render(img_area, buf);
                        Span::styled(NO_IMAGE, neutral_style).render(
                            utils::align(
                                Rect {
                                    width: NO_IMAGE.len() as u16,
                                    height: 1,
                                    ..img_area
                                },
                                img_area,
                                utils::Alignment::Center,
                            ),
                            buf,
                        );
                    }
                    FrontCover::Loading => {
                        const LOADING: &str = "LOADING";
                        Block::bordered().style(neutral_style).render(img_area, buf);
                        Span::styled(LOADING, neutral_style).render(
                            utils::align(
                                Rect {
                                    width: LOADING.len() as u16,
                                    height: 1,
                                    ..img_area
                                },
                                img_area,
                                utils::Alignment::Center,
                            ),
                            buf,
                        );
                    }
                    FrontCover::Ready(image) => {
                        let resized_img_area =
                            image.size_for(ratatui_image::Resize::default(), img_area);
                        img_area = utils::align(
                            Rect {
                                width: resized_img_area.width,
                                height: resized_img_area.height,
                                ..left_area
                            },
                            left_area,
                            utils::Alignment::Right,
                        );
                        StatefulImage::default().render(img_area, buf, image);
                    }
                }

                let mut info_area = utils::align(
                    Rect {
                        height: 11,
                        ..right_area
                    },
                    img_area,
                    utils::Alignment::CenterVertical,
                );

                let track = jb.get(id).unwrap();

                info_area.height = 1;

                for (label, info) in [
                    ("ALBUM", track.album()),
                    ("TITLE", track.title()),
                    ("ARTIST", track.artist()),
                    ("RATING", track.rating_display()),
                ] {
                    Span::styled(label, neutral_style).render(info_area, buf);
                    info_area.y += 1;
                    Span::raw(info).render(info_area, buf);
                    info_area.y += 2;
                }
            }
            None => {
                const NO_TRACK: &str = "No track currently playing";
                Span::styled(NO_TRACK, neutral_style).render(
                    utils::align(
                        Rect {
                            width: NO_TRACK.len() as u16,
                            height: 1,
                            ..area
                        },
                        area,
                        utils::Alignment::CenterHorizontal,
                    ),
                    buf,
                );
            }
        }
    }

    pub fn on_input(&mut self, _key: KeyCode, _modifiers: KeyModifiers) {}

    pub fn on_exit(&mut self) {}
}

enum FrontCover {
    None,
    Loading,
    Ready(StatefulProtocol),
}
