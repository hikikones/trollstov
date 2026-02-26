use jukebox::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Action,
    colors::Colors,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, utils},
};

pub struct SettingsPage {
    list: List,
    skip_rating: AudioRating,
}

enum Setting {
    SkipRating,
    Ignore,
    Test,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Setting::SkipRating => true,
            Setting::Ignore => false,
            Setting::Test => true,
        }
    }
}

const SETTINGS: [Setting; 8] = [
    Setting::SkipRating,
    Setting::Ignore,
    Setting::Test,
    Setting::Test,
    Setting::Test,
    Setting::Test,
    Setting::Ignore,
    Setting::SkipRating,
];

impl SettingsPage {
    pub const fn new(colors: &Colors) -> Self {
        Self {
            list: List::new().with_colors(colors.neutral, None),
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
        self.list.render(
            area,
            buf,
            SETTINGS.into_iter(),
            |line, buf, setting, index| {
                let (symbol, style) = if index == ListItem::Selected {
                    ("> ", Style::new())
                } else {
                    ("", Style::new())
                };

                match setting {
                    Setting::SkipRating => {
                        let line = utils::_print_line_iter(
                            line,
                            buf,
                            [symbol, "Skip tracks with rating up to: "],
                            style,
                        );
                        print_stars(line, buf, self.skip_rating, colors);
                    }
                    Setting::Ignore => {
                        Span::raw("ignore......").render(line, buf);
                    }
                    Setting::Test => {
                        utils::_print_line_iter(line, buf, [symbol, "Test section"], style);
                    }
                }
            },
        );

        // Shortcuts
        shortcuts.extend([Shortcut::new("Rating", "0-5")]);

        shortcuts.push(Shortcut::new("Save", "^s"));
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);

        match key {
            KeyCode::Down => {
                if let Some((i, _)) = SETTINGS
                    .into_iter()
                    .enumerate()
                    .skip(self.list.index() + 1)
                    .filter(|(_, s)| s.filter())
                    .next()
                {
                    if self.list.move_index(ListMove::Custom(i), false) {
                        return Action::Render;
                    }
                }
            }
            KeyCode::Up => {
                let index_rev = SETTINGS.len().saturating_sub(1) - self.list.index();
                if let Some((i, _)) = SETTINGS
                    .into_iter()
                    .enumerate()
                    .rev()
                    .skip(index_rev + 1)
                    .filter(|(_, s)| s.filter())
                    .next()
                {
                    if self.list.move_index(ListMove::Custom(i), false) {
                        return Action::Render;
                    }
                }
            }
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

fn print_stars(mut line: Rect, buf: &mut Buffer, rating: AudioRating, colors: &Colors) {
    let colored = rating as u8;
    let neutral = 5 - colored;
    line = utils::_print_line_iter(line, buf, (0..colored).map(|_| "★"), colors.accent);
    utils::_print_line_iter(line, buf, (0..neutral).map(|_| "★"), colors.neutral);
}
