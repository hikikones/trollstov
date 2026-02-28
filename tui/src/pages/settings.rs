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
    accent: ColorSetting,
    on_accent: ColorSetting,
    neutral: ColorSetting,
    on_neutral: ColorSetting,
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
    OnAccentColor,
    OnAccentColorDescription,
    NeutralColor,
    NeutralColorDescription,
    OnNeutralColor,
    OnNeutralColorDescription,
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
            Setting::OnAccentColor => true,
            Setting::OnAccentColorDescription => false,
            Setting::NeutralColor => true,
            Setting::NeutralColorDescription => false,
            Setting::OnNeutralColor => true,
            Setting::OnNeutralColorDescription => false,
            Setting::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 23] = [
    Setting::Empty,
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
    Setting::Empty,
    Setting::OnAccentColor,
    Setting::OnAccentColorDescription,
    Setting::Empty,
    Setting::NeutralColor,
    Setting::NeutralColorDescription,
    Setting::Empty,
    Setting::OnNeutralColor,
    Setting::OnNeutralColorDescription,
    Setting::Empty,
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
            list: List::new().with_index(3).with_margins(5, 5),
            text: TextSegment::new().with_alignment(Alignment::Center),
            accent: ColorSetting::new(settings.accent().to_string()),
            on_accent: ColorSetting::new(settings.on_accent().to_string()),
            neutral: ColorSetting::new(settings.neutral().to_string()),
            on_neutral: ColorSetting::new(settings.on_neutral().to_string()),
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
            .padding(Padding::horizontal(1));
        let settings_area = block.inner(area);
        block.render(area, buf);

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
                        let s = "Skip tracks with rating: ";
                        self.text.extend_as_one([symbol, s], style);

                        // Stars
                        let colored = self.settings.skip_rating() as usize;
                        let neutral = 5 - colored;
                        self.text.repeat_char('★', colored, settings.accent());
                        self.text.repeat_char('★', neutral, settings.neutral());
                        self.text.render(line, buf);
                    }
                    Setting::SkipRatingDescription => {
                        let s = "skips tracks that are less than or equal to";
                        self.text.push_str(s, settings.neutral());
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSort => {
                        let s = "Keep selected track on sort: ";
                        self.text.extend_as_one([symbol, s], style);

                        // Checkmark
                        let (checkmark, color) = match self.settings.keep_on_sort() {
                            true => ("🗸", settings.accent()),
                            false => ("𐄂", settings.neutral()),
                        };
                        self.text.push_str(checkmark, color);
                        self.text.render(line, buf);
                    }
                    Setting::KeepTrackSortDescription => {
                        let s = "scrolls to selected track when sorting";
                        self.text.push_str(s, settings.neutral());
                        self.text.render(line, buf);
                    }
                    Setting::Colors => {
                        self.text.push_str("COLORS", style);
                        self.text.render(line, buf);
                    }
                    Setting::AccentColor => {
                        let s = "Set accent color: ";
                        let is_active = current == Setting::AccentColor;
                        self.accent.set_active(is_active, settings);
                        self.accent.render(line, buf, &[symbol, s]);
                    }
                    Setting::AccentColorDescription => {
                        let style = Style::new().fg(self.settings.accent());
                        self.text.push_str("accent color", style);
                        self.text.render(line, buf);
                    }
                    Setting::OnAccentColor => {
                        let s = "Set on accent color: ";
                        let is_active = current == Setting::OnAccentColor;
                        self.on_accent.set_active(is_active, settings);
                        self.on_accent.render(line, buf, &[symbol, s]);
                    }
                    Setting::OnAccentColorDescription => {
                        let style = Style::new()
                            .bg(self.settings.accent())
                            .fg(self.settings.on_accent());
                        self.text.push_str(" on accent color ", style);
                        self.text.render(line, buf);
                    }
                    Setting::NeutralColor => {
                        let s = "Set neutral color: ";
                        let is_active = current == Setting::NeutralColor;
                        self.neutral.set_active(is_active, settings);
                        self.neutral.render(line, buf, &[symbol, s]);
                    }
                    Setting::NeutralColorDescription => {
                        let style = Style::new().fg(self.settings.neutral());
                        self.text.push_str("neutral color", style);
                        self.text.render(line, buf);
                    }
                    Setting::OnNeutralColor => {
                        let s = "Set accent color: ";
                        let is_active = current == Setting::OnNeutralColor;
                        self.on_neutral.set_active(is_active, settings);
                        self.on_neutral.render(line, buf, &[symbol, s]);
                    }
                    Setting::OnNeutralColorDescription => {
                        let style = Style::new()
                            .bg(self.settings.neutral())
                            .fg(self.settings.on_neutral());
                        self.text.push_str(" on neutral color ", style);
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
            Setting::AccentColor
            | Setting::OnAccentColor
            | Setting::NeutralColor
            | Setting::OnNeutralColor => {
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
                    match self.accent.parse_color() {
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
                } else if self.accent.input(key, modifiers) {
                    return Action::Render;
                }
            }
            Setting::OnAccentColor => {
                if let KeyCode::Enter = key {
                    match self.on_accent.parse_color() {
                        Ok(color) => {
                            if self.settings.on_accent() != color {
                                self.settings.set_on_accent(color);
                                self.update_hash();
                                return Action::Render;
                            }
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            return Action::Log(log);
                        }
                    }
                } else if self.on_accent.input(key, modifiers) {
                    return Action::Render;
                }
            }
            Setting::NeutralColor => {
                if let KeyCode::Enter = key {
                    match self.neutral.parse_color() {
                        Ok(color) => {
                            if self.settings.neutral() != color {
                                self.settings.set_neutral(color);
                                self.update_hash();
                                return Action::Render;
                            }
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            return Action::Log(log);
                        }
                    }
                } else if self.neutral.input(key, modifiers) {
                    return Action::Render;
                }
            }
            Setting::OnNeutralColor => {
                if let KeyCode::Enter = key {
                    match self.on_neutral.parse_color() {
                        Ok(color) => {
                            if self.settings.on_neutral() != color {
                                self.settings.set_on_neutral(color);
                                self.update_hash();
                                return Action::Render;
                            }
                        }
                        Err(err) => {
                            let log = Log::new(err);
                            return Action::Log(log);
                        }
                    }
                } else if self.on_neutral.input(key, modifiers) {
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

struct ColorSetting(TextInput);

impl ColorSetting {
    const fn new(s: String) -> Self {
        Self(TextInput::from(s))
    }

    fn parse_color(&self) -> Result<Color, ratatui::style::ParseColorError> {
        Color::from_str(self.0.as_str().trim())
    }

    const fn set_active(&mut self, active: bool, settings: &Settings) {
        let styles = if active {
            TextInputStyles {
                normal: Style::new(),
                cursor: Style::new().bg(settings.accent()).fg(settings.on_accent()),
                selector: Style::new()
                    .bg(settings.neutral())
                    .fg(settings.on_neutral()),
                placeholder: Style::new(),
            }
        } else {
            TextInputStyles::all(Style::new())
        };
        self.0.set_styles(styles);
    }

    fn input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        self.0.input(key, modifiers)
    }

    fn render(&mut self, line: Rect, buf: &mut Buffer, texts: &[&str]) {
        const INPUT_WIDTH: u16 = 10;

        let width = texts.iter().map(|s| s.len() as u16).sum::<u16>() + INPUT_WIDTH;
        let mut line = utils::align(
            Rect { width, ..line },
            line,
            utils::Alignment::CenterHorizontal,
        );

        line = utils::print_asciis_simple(line, buf, texts, Style::new());
        self.0.render(line, buf);
    }
}
