use jukebox::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    colors::Colors,
    settings::Settings,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, TextSegment},
};

pub struct SettingsPage {
    list: List,
    text: TextSegment,
    skip_rating: AudioRating,
}

enum Setting {
    SkipRating,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Setting::SkipRating => true,
        }
    }
}

const SETTINGS: [Setting; 1] = [Setting::SkipRating];

impl SettingsPage {
    pub const fn new(colors: &Colors) -> Self {
        Self {
            list: List::new().with_colors(colors.neutral, None),
            text: TextSegment::new().with_alignment(Alignment::Center),
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
        let area = area.centered_horizontally(Constraint::Max(80));

        let block = Block::bordered()
            .title(" Settings ")
            .title_alignment(Alignment::Center)
            .padding(Padding::uniform(1));
        let settings_area = block.inner(area);

        block.render(area, buf);

        self.list.render(
            settings_area,
            buf,
            SETTINGS.into_iter(),
            |line, buf, setting, index| {
                let (symbol, style) = if index == ListItem::Selected {
                    ("> ", Style::new())
                } else {
                    ("", Style::new().fg(colors.neutral))
                };

                match setting {
                    Setting::SkipRating => {
                        self.text
                            .extend_as_one([symbol, "Skip tracks with rating up to: "], style);

                        // Stars
                        let colored = self.skip_rating as usize;
                        let neutral = 5 - colored;
                        self.text.repeat_char('★', colored, colors.accent);
                        self.text.repeat_char('★', neutral, colors.neutral);
                        self.text.render(line, buf);
                    }
                }

                self.text.clear();
            },
        );

        // Shortcuts
        match SETTINGS[self.list.index()] {
            Setting::SkipRating => {
                shortcuts.extend([Shortcut::new("Rating", "0-5")]);
            }
        }
        shortcuts.push(Shortcut::new("Save", "^s"));
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        settings: &Settings,
    ) -> Action {
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
