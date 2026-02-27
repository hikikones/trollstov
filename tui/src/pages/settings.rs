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

        match SETTINGS[self.list.index()] {
            Setting::SkipRating => {
                shortcuts.extend([Shortcut::new("Rating", "0-5")]);
            }
            Setting::Ignore => {}
            Setting::Test => {}
        }
        shortcuts.push(Shortcut::new("Save", "^s"));
    }

    pub fn on_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);

        match key {
            KeyCode::Down => {
                let mut next = self.list.index() + 1;
                while next < SETTINGS.len() {
                    if SETTINGS[next].filter() {
                        if self.list.move_index(ListMove::Custom(next), false) {
                            return Action::Render;
                        } else {
                            break;
                        }
                    }
                    next += 1;
                }
            }
            KeyCode::Up => {
                let mut prev = self.list.index().saturating_sub(1);
                loop {
                    if SETTINGS[prev].filter() {
                        if self.list.move_index(ListMove::Custom(prev), false) {
                            return Action::Render;
                        } else {
                            break;
                        }
                    }

                    if prev == 0 {
                        break;
                    }

                    prev -= 1;
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
    line = utils::print_text_repeat(line, buf, "★", colored, colors.accent);
    utils::print_text_repeat(line, buf, "★", neutral, colors.neutral);
}
