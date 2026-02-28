use std::str::FromStr;

use jukebox::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};

use crate::{
    app::Action,
    pages::Log,
    settings::Settings,
    widgets::{
        List, ListItem, ListMove, Shortcut, Shortcuts, TextInput, TextInputStyles, TextSegment,
        utils,
    },
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
    accent_input: TextInput,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Setting {
    General,
    SkipRating,
    SkipRatingDescription,
    KeepTrackSort,
    KeepTrackSortDescription,
    Colors,
    AccentColor,
    AccentColorDescription,
    Empty,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Setting::General => false,
            Setting::SkipRating => true,
            Setting::SkipRatingDescription => false,
            Setting::KeepTrackSort => true,
            Setting::KeepTrackSortDescription => false,
            Setting::Colors => false,
            Setting::AccentColor => true,
            Setting::AccentColorDescription => false,
            Setting::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 12] = [
    Setting::General,
    Setting::Empty,
    Setting::SkipRating,
    Setting::SkipRatingDescription,
    Setting::Empty,
    Setting::KeepTrackSort,
    Setting::KeepTrackSortDescription,
    Setting::Empty,
    Setting::Colors,
    Setting::Empty,
    Setting::AccentColor,
    Setting::AccentColorDescription,
];

impl SettingsPage {
    pub fn new(settings: &Settings) -> Self {
        let hash = settings.hash();
        Self {
            settings: settings.clone(),
            applied: settings.clone(),
            written: settings.clone(),
            apply_hash: hash,
            write_hash: hash,
            is_applied: true,
            is_written: true,
            list: List::new().with_index(2),
            text: TextSegment::new().with_alignment(Alignment::Center),
            accent_input: TextInput::from(settings.accent().to_string()),
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        settings: &Settings,
        shortcuts: &mut Shortcuts,
    ) {
        let area = area.centered_horizontally(Constraint::Max(80));

        let block = Block::bordered()
            .title(" Settings ")
            .title_alignment(Alignment::Center)
            .padding(Padding::uniform(1));
        let settings_area = block.inner(area);

        block.render(area, buf);

        self.accent_input
            .set_styles(TextInputStyles::all(Style::new()));
        let current = self.current();

        self.list.set_colors(settings.neutral(), None).render(
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
                    Setting::General => {
                        self.text.push_str("GENERAL", style);
                        self.text.render(line, buf);
                    }
                    Setting::SkipRating => {
                        self.text
                            .extend_as_one([symbol, "Skip tracks with rating: "], style);

                        // Stars
                        let colored = self.settings.skip_rating() as usize;
                        let neutral = 5 - colored;
                        self.text.repeat_char('★', colored, settings.accent());
                        self.text.repeat_char('★', neutral, settings.neutral());
                        self.text.render(line, buf);
                    }
                    Setting::SkipRatingDescription => {
                        self.text.push_str(
                            "skips tracks that are less than or equal to",
                            settings.neutral(),
                        );
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSort => {
                        self.text
                            .extend_as_one([symbol, "Keep selected track on sort: "], style);

                        // Checkmark
                        let (checkmark, color) = match self.settings.keep_on_sort() {
                            true => ("🗸", settings.accent()),
                            false => ("𐄂", settings.neutral()),
                        };
                        self.text.push_str(checkmark, color);
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSortDescription => {
                        self.text
                            .push_str("scrolls to selected track when sorting", settings.neutral());
                        self.text.render(line, buf);
                    }
                    Setting::Colors => {
                        self.text.push_str("COLORS", style);
                        self.text.render(line, buf);
                    }
                    Setting::AccentColor => {
                        let s = "Set accent color: ";
                        let spacing = "        ";
                        // let n = symbol.len() + s.len() + spacing.len();
                        // let sa = utils::align(
                        //     Rect {
                        //         width: n as u16,
                        //         ..line
                        //     },
                        //     line,
                        //     utils::Alignment::CenterHorizontal,
                        // );
                        // utils::print_ascii(sa, buf, s, style, utils::Alignment::Left);
                        let mut line = utils::print_ascii_iter(
                            line,
                            buf,
                            &[symbol, s, spacing],
                            style,
                            utils::Alignment::CenterHorizontal,
                        );
                        line.x = line.x.saturating_sub(spacing.len() as u16);
                        line.width += spacing.len() as u16;
                        if current == Setting::AccentColor {
                            self.accent_input.set_styles(TextInputStyles {
                                normal: Style::new(),
                                cursor: Style::new().bg(settings.accent()).fg(settings.on_accent()),
                                selector: Style::new()
                                    .bg(settings.neutral())
                                    .fg(settings.on_neutral()),
                                placeholder: Style::new(),
                            });
                        }
                        self.accent_input.render(line, buf);

                        // self.text
                        //     .extend_as_one([symbol, "Set accent color:         "], style);
                        // self.text.render(line, buf);
                    }
                    Setting::AccentColorDescription => {
                        self.text.push_str("accent color", self.settings.accent());
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
            Setting::AccentColor => {
                shortcuts.push(Shortcut::new("Set color", "↵"));
            }
            _ => {}
        }

        if !self.is_applied {
            shortcuts.push(Shortcut::new("Apply", "^a"));
        }
        if !self.is_written {
            shortcuts.push(Shortcut::new("Save", "^s"));
        }
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        settings: &mut Settings,
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
                'a' => {
                    if ctrl && !self.is_applied {
                        *settings = self.settings.clone();
                        self.applied = self.settings.clone();
                        self.apply_hash = self.settings.hash();
                        self.is_applied = true;
                        return Action::ApplySettings;
                    } else {
                        return self.handle_setting(key, modifiers);
                    }
                }
                's' => {
                    if ctrl && !self.is_written {
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
                    } else {
                        return self.handle_setting(key, modifiers);
                    }
                }
                _ => return self.handle_setting(key, modifiers),
            },
            _ => return self.handle_setting(key, modifiers),
        }

        Action::None
    }

    pub fn on_exit(&self) {}

    fn handle_setting(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Action {
        match self.current() {
            Setting::SkipRating => {
                if let KeyCode::Char(c) = key
                    && let Some(rating) = AudioRating::from_char(c)
                    && self.settings.skip_rating() != rating
                {
                    self.settings.set_skip_rating(rating);
                    self.update_hash();
                    return Action::Render;
                }
            }
            Setting::KeepTrackSort => {
                if let KeyCode::Char(' ') = key {
                    let toggle = !self.settings.keep_on_sort();
                    self.settings.set_keep_on_sort(toggle);
                    self.update_hash();
                    return Action::Render;
                }
            }
            Setting::AccentColor => {
                if let KeyCode::Enter = key {
                    match Color::from_str(self.accent_input.as_str().trim()) {
                        Ok(color) => {
                            if self.settings.accent() != color {
                                self.settings.set_accent(color);
                                self.update_hash();
                                return Action::Render;
                            }
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            return Action::Log(log);
                        }
                    }
                } else if self.accent_input.input(key, modifiers) {
                    return Action::Render;
                }
            }
            _ => {}
        }

        Action::None
    }

    fn update_hash(&mut self) {
        let hash = self.settings.hash();
        self.is_applied = self.apply_hash == hash;
        self.is_written = self.write_hash == hash;
    }

    const fn current(&self) -> Setting {
        SETTINGS[self.list.index()]
    }
}
