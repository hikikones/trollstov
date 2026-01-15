use std::thread::JoinHandle;

use crossterm::event::{KeyCode, KeyModifiers};
use image::GenericImageView;
use ratatui::{prelude::*, widgets::Block};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use crate::{
    app::Colors,
    audio::AudioPicture,
    events::EventHandler,
    jukebox::{Jukebox, TrackId},
};

pub struct NowPlayingPage {
    current: Option<TrackId>,
    picker: Picker,
    image: FrontCover,
    image_handle: Option<JoinHandle<FrontCover>>,
}

impl NowPlayingPage {
    pub fn new() -> Self {
        Self {
            current: None,
            picker: Picker::from_query_stdio().unwrap(),
            image: FrontCover::None,
            image_handle: None,
        }
    }

    pub fn on_enter(&mut self, jb: &Jukebox) {
        // todo?
    }

    pub fn on_update(&mut self, jb: &Jukebox) {
        if self.current != jb.current() {
            self.current = jb.current();

            // Track has changed, time to update image
            match jb.current() {
                Some(tid) => {
                    // Load image in thread and store handle
                    self.image = FrontCover::Loading;
                    let path = jb.get(tid).unwrap().path().to_path_buf();
                    let picker = self.picker.clone();
                    let handle = std::thread::spawn(move || {
                        let picture = AudioPicture::read(path).unwrap();
                        match picture.bytes() {
                            Some(bytes) => {
                                const MAX: u32 = 720;
                                let mut dyn_img = image::load_from_memory(bytes).unwrap();
                                let (w, h) = dyn_img.dimensions();
                                if w > MAX || h > MAX {
                                    dyn_img = dyn_img.thumbnail(MAX, MAX);
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
                const MAX_IMAGE_SIZE: u16 = 15;
                let [left_area, _, right_area] = Layout::horizontal([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Fill(1),
                ])
                .areas(area);

                let mut img_area = {
                    let img_w = left_area.width.min(MAX_IMAGE_SIZE * 2);
                    let img_h = left_area.height.min(MAX_IMAGE_SIZE);
                    Rect {
                        width: img_w,
                        height: img_h,
                        x: left_area.x + left_area.width.saturating_sub(img_w),
                        ..left_area
                    }
                };

                match &mut self.image {
                    FrontCover::None => {
                        Block::bordered().style(neutral_style).render(img_area, buf);
                        Line::styled("NO IMAGE", neutral_style).centered().render(
                            img_area.centered(Constraint::Length(8), Constraint::Length(1)),
                            buf,
                        );
                    }
                    FrontCover::Loading => {
                        Block::bordered().style(neutral_style).render(img_area, buf);
                        Line::styled("LOADING", neutral_style).centered().render(
                            img_area.centered(Constraint::Length(7), Constraint::Length(1)),
                            buf,
                        );
                    }
                    FrontCover::Ready(image) => {
                        let new_img_area =
                            image.size_for(ratatui_image::Resize::default(), img_area);
                        img_area = Rect {
                            x: left_area.x + left_area.width.saturating_sub(new_img_area.width),
                            y: left_area.y,
                            width: new_img_area.width,
                            height: new_img_area.height,
                        };
                        StatefulImage::default().render(img_area, buf, image);
                    }
                }

                let info_area = Rect {
                    height: img_area.height,
                    ..right_area
                }
                .centered_vertically(Constraint::Length(8));

                let [
                    album_label,
                    album_info,
                    _,
                    title_label,
                    title_info,
                    _,
                    artist_label,
                    artist_info,
                ] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .areas(info_area);

                let track = jb.get(id).unwrap();

                Line::styled("ALBUM", neutral_style).render(album_label, buf);
                Line::raw(track.album()).render(album_info, buf);

                Line::styled("TITLE", neutral_style).render(title_label, buf);
                Line::raw(track.title()).render(title_info, buf);

                Line::styled("ARTIST", neutral_style).render(artist_label, buf);
                Line::raw(track.artist()).render(artist_info, buf);
            }
            None => {
                Line::styled("No track currently playing", neutral_style)
                    .centered()
                    .render(area, buf);
            }
        }
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers, events: &EventHandler) {
        // todo
    }

    pub fn on_exit(&mut self) {
        // todo
    }
}

enum FrontCover {
    None,
    Loading,
    Ready(StatefulProtocol),
}
