use jukebox::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Action,
    colors::Colors,
    widgets::{Shortcut, Shortcuts, utils},
};

pub struct SettingsPage {
    skip_rating: AudioRating,
}

impl SettingsPage {
    pub const fn new(colors: &Colors) -> Self {
        Self {
            skip_rating: AudioRating::None,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        // Skip rating
        print_stars(area, buf, self.skip_rating, colors);

        // Shortcuts
        shortcuts.extend([Shortcut::new("Rating", "0-5"), Shortcut::new("Save", "^s")]);
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);

        match key {
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    self.skip_rating = AudioRating::from_char(c).unwrap();
                    return Action::Render;
                }
                's' => {
                    if ctrl {
                        //todo save
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Action::None
    }

    pub fn on_exit(&self) {}
}

fn print_stars(mut area: Rect, buf: &mut Buffer, rating: AudioRating, colors: &Colors) {
    let colored = rating as u8;
    let neutral = 5 - colored;
    area = utils::_print_line_iter(area, buf, (0..colored).map(|_| "★"), colors.accent);
    utils::_print_line_iter(area, buf, (0..neutral).map(|_| "★"), colors.neutral);
}
