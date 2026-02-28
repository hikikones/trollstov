use jukebox::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    colors::Colors,
    pages::Log,
    settings::Settings,
    widgets::{List, ListItem, ListMove, Shortcut, Shortcuts, TextSegment},
};

pub struct SettingsPage {
    settings: Settings,
    applied: Settings,
    written: Settings,
    apply_hash: u64,
    write_hash: u64,
    is_applied: bool,
    is_written: bool,
    list: List,
    text: TextSegment,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Setting {
    SkipRating,
    SkipRatingDescription,
    KeepTrackSort,
    KeepTrackSortDescription,
    Empty,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Setting::SkipRating => true,
            Setting::SkipRatingDescription => false,
            Setting::KeepTrackSort => true,
            Setting::KeepTrackSortDescription => false,
            Setting::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 5] = [
    Setting::SkipRating,
    Setting::SkipRatingDescription,
    Setting::Empty,
    Setting::KeepTrackSort,
    Setting::KeepTrackSortDescription,
];

impl SettingsPage {
    pub fn new(settings: &Settings, colors: &Colors) -> Self {
        let hash = settings.hash();
        Self {
            settings: settings.clone(),
            applied: settings.clone(),
            written: settings.clone(),
            apply_hash: hash,
            write_hash: hash,
            is_applied: true,
            is_written: true,
            list: List::new().with_colors(colors.neutral, None),
            text: TextSegment::new().with_alignment(Alignment::Center),
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
                    ("> ", Style::new().bold())
                } else {
                    ("", Style::new())
                };

                match setting {
                    Setting::SkipRating => {
                        self.text
                            .extend_as_one([symbol, "Skip tracks with rating: "], style);

                        // Stars
                        let colored = self.settings.skip_rating() as usize;
                        let neutral = 5 - colored;
                        self.text.repeat_char('★', colored, colors.accent);
                        self.text.repeat_char('★', neutral, colors.neutral);
                        self.text.render(line, buf);
                    }
                    Setting::SkipRatingDescription => {
                        self.text.push_str(
                            "skips tracks that are less than or equal to",
                            colors.neutral,
                        );
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSort => {
                        self.text
                            .extend_as_one([symbol, "Keep selected track on sort: "], style);

                        // Checkmark
                        let (checkmark, color) = match self.settings.keep_on_sort() {
                            true => ("🗸", colors.accent),
                            false => ("𐄂", colors.neutral),
                        };
                        self.text.push_str(checkmark, color);
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSortDescription => {
                        self.text
                            .push_str("scrolls to selected track when sorting", colors.neutral);
                        self.text.render(line, buf);
                    }
                    Setting::Empty => {}
                }

                self.text.clear();
            },
        );

        // Shortcuts
        match SETTINGS[self.list.index()] {
            Setting::SkipRating => {
                shortcuts.push(Shortcut::new("Rating", "0-5"));
            }
            Setting::KeepTrackSort => {
                shortcuts.push(Shortcut::new("Toggle", "space"));
            }
            _ => {}
        }

        if !self.is_applied {
            shortcuts.push(Shortcut::new("Apply", "a"));
        }
        if !self.is_written {
            shortcuts.push(Shortcut::new("Save", "s"));
        }
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        settings: &mut Settings,
    ) -> Action {
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
                    if self.current() == Setting::SkipRating {
                        let rating = AudioRating::from_char(c).unwrap();
                        if self.settings.skip_rating() != rating {
                            self.settings.set_skip_rating(rating);
                            self.update_hash();
                            return Action::Render;
                        }
                    }
                }
                ' ' => {
                    if self.current() == Setting::KeepTrackSort {
                        let toggle = !self.settings.keep_on_sort();
                        self.settings.set_keep_on_sort(toggle);
                        self.update_hash();
                        return Action::Render;
                    }
                }
                'a' => {
                    if !self.is_applied {
                        *settings = self.settings.clone();
                        self.applied = self.settings.clone();
                        self.apply_hash = self.settings.hash();
                        self.is_applied = true;
                        return Action::ApplySettings;
                    }
                }
                's' => {
                    if !self.is_written {
                        match settings.save() {
                            Ok(_) => {
                                self.written = self.settings.clone();
                                self.write_hash = self.settings.hash();
                                self.is_written = true;
                                return Action::Render;
                            }
                            Err(err) => {
                                return Action::Log(Log::new(err));
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn update_hash(&mut self) {
        let hash = self.settings.hash();
        self.is_applied = self.apply_hash == hash;
        self.is_written = self.write_hash == hash;
    }

    const fn current(&self) -> Setting {
        SETTINGS[self.list.index()]
    }
}
