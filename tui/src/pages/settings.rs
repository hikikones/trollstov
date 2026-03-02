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
    settings::{Colors, Settings},
    widgets::{
        List, ListItem, ListMove, Shortcut, Shortcuts, TextInput, TextInputStyles, TextSegment,
        utils,
    },
};

// TODO: Show description for each setting on a dedicated line.
// Probably better than being part of the list.

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
    ColorsDescription,
    AccentColor,
    AccentColorPreview,
    OnAccentColor,
    OnAccentColorPreview,
    NeutralColor,
    NeutralColorPreview,
    OnNeutralColor,
    OnNeutralColorPreview,
    Empty,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Self::General => false,
            Self::SkipRating => true,
            Self::SkipRatingDescription => false,
            Self::KeepTrackSort => true,
            Self::KeepTrackSortDescription => false,
            Self::Colors => false,
            Self::ColorsDescription => false,
            Self::AccentColor => true,
            Self::AccentColorPreview => false,
            Self::OnAccentColor => true,
            Self::OnAccentColorPreview => false,
            Self::NeutralColor => true,
            Self::NeutralColorPreview => false,
            Self::OnNeutralColor => true,
            Self::OnNeutralColorPreview => false,
            Self::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 24] = [
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
    Setting::ColorsDescription,
    Setting::Empty,
    Setting::AccentColor,
    Setting::AccentColorPreview,
    Setting::Empty,
    Setting::OnAccentColor,
    Setting::OnAccentColorPreview,
    Setting::Empty,
    Setting::NeutralColor,
    Setting::NeutralColorPreview,
    Setting::Empty,
    Setting::OnNeutralColor,
    Setting::OnNeutralColorPreview,
    Setting::Empty,
];

impl SettingsPage {
    pub fn new(settings: &Settings) -> Self {
        let colors = settings.colors();
        let hash = settings.hash();
        let selected = if SETTINGS[0].filter() {
            0
        } else {
            next(0).unwrap()
        };

        Self {
            settings: settings.clone(),
            applied: settings.clone(),
            written: settings.clone(),
            apply_hash: hash,
            write_hash: hash,
            is_applied: true,
            is_written: true,
            list: List::new().with_index(selected).with_margins(5, 5),
            text: TextSegment::new().with_alignment(Alignment::Center),
            accent: ColorSetting::new(colors.accent),
            on_accent: ColorSetting::new(colors.on_accent),
            neutral: ColorSetting::new(colors.neutral),
            on_neutral: ColorSetting::new(colors.on_neutral),
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

        let mut label_area = Rect {
            width: settings_area.width / 2,
            ..settings_area
        };
        let mut setting_area = Rect {
            x: label_area.x + label_area.width + 1,
            width: label_area.width.saturating_sub(1),
            ..label_area
        };

        let colors = settings.colors();
        let current = self.current();

        self.list.set_colors(colors.neutral, None).render(
            settings_area,
            buf,
            SETTINGS,
            |line, buf, setting, index| {
                label_area.y = line.y;
                setting_area.y = line.y;

                let (symbol, style) = if index == ListItem::Selected {
                    ("> ", Style::new().bold())
                } else {
                    ("", Style::new())
                };

                match setting {
                    Setting::General => {
                        utils::print_ascii(
                            line,
                            buf,
                            "GENERAL",
                            style,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::SkipRating => {
                        let s = "Skip tracks with rating:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        // Stars
                        let colored = self.settings.skip_rating().as_u8();
                        let neutral = 5 - colored;
                        let setting_area = utils::print_char_repeat(
                            setting_area,
                            buf,
                            '★',
                            colored,
                            colors.accent,
                        );
                        utils::print_char_repeat(setting_area, buf, '★', neutral, colors.neutral);
                    }
                    Setting::SkipRatingDescription => {
                        utils::print_ascii(
                            line,
                            buf,
                            "skips tracks that are less than or equal to",
                            colors.neutral,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::KeepTrackSort => {
                        let s = "Keep selected track on sort:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        // Checkmark
                        let (checkmark, color) = match self.settings.keep_on_sort() {
                            true => ('🗸', colors.accent),
                            false => ('𐄂', colors.neutral),
                        };
                        let Rect { x, y, .. } = setting_area;
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_char(checkmark).set_style(color);
                        }
                    }
                    Setting::KeepTrackSortDescription => {
                        utils::print_ascii(
                            line,
                            buf,
                            "scrolls to selected track when sorting",
                            colors.neutral,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::Colors => {
                        utils::print_ascii(
                            line,
                            buf,
                            "COLORS",
                            style,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::ColorsDescription => {
                        utils::print_ascii(
                            line,
                            buf,
                            "set colors by name, hex code or indexed value",
                            colors.neutral,
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::AccentColor => {
                        let s = "Set accent color:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        let is_active = current == Setting::AccentColor;
                        self.accent.set_active(is_active, colors);
                        self.accent.render(setting_area, buf);
                    }
                    Setting::AccentColorPreview => {
                        utils::print_ascii(
                            line,
                            buf,
                            "accent color",
                            Style::new().fg(self.settings.accent()),
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::OnAccentColor => {
                        let s = "Set on accent color:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        let is_active = current == Setting::OnAccentColor;
                        self.on_accent.set_active(is_active, colors);
                        self.on_accent.render(setting_area, buf);
                    }
                    Setting::OnAccentColorPreview => {
                        utils::print_ascii(
                            line,
                            buf,
                            " on accent color ",
                            Style::new()
                                .bg(self.settings.accent())
                                .fg(self.settings.on_accent()),
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::NeutralColor => {
                        let s = "Set neutral color:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        let is_active = current == Setting::NeutralColor;
                        self.neutral.set_active(is_active, colors);
                        self.neutral.render(setting_area, buf);
                    }
                    Setting::NeutralColorPreview => {
                        utils::print_ascii(
                            line,
                            buf,
                            "neutral color",
                            Style::new().fg(self.settings.neutral()),
                            Some(utils::Alignment::CenterHorizontal),
                        );
                    }
                    Setting::OnNeutralColor => {
                        let s = "Set on neutral color:";
                        utils::print_asciis(
                            label_area,
                            buf,
                            [symbol, s],
                            style,
                            Some(utils::Alignment::Right),
                        );

                        let is_active = current == Setting::OnNeutralColor;
                        self.on_neutral.set_active(is_active, colors);
                        self.on_neutral.render(setting_area, buf);
                    }
                    Setting::OnNeutralColorPreview => {
                        utils::print_ascii(
                            line,
                            buf,
                            " on neutral color ",
                            Style::new()
                                .bg(self.settings.neutral())
                                .fg(self.settings.on_neutral()),
                            Some(utils::Alignment::CenterHorizontal),
                        );
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
            Setting::AccentColor | Setting::NeutralColor => {
                shortcuts.push(Shortcut::new("Set color", "↵"));
            }
            Setting::OnAccentColor | Setting::OnNeutralColor => {
                shortcuts.extend([
                    Shortcut::new("Set color", "↵"),
                    Shortcut::new("Generate color", "^g"),
                ]);
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
                if let Some(next) = next(self.list.index()) {
                    self.list.move_index(ListMove::Custom(next), false);
                    return Action::Render;
                }
            }
            KeyCode::Up => {
                if let Some(prev) = previous(self.list.index()) {
                    self.list.move_index(ListMove::Custom(prev), false);
                    return Action::Render;
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
                } else if modifiers.contains(KeyModifiers::CONTROL)
                    && let KeyCode::Char('g') = key
                {
                    let bg = self.settings.accent();
                    let fg = Colors::generate_readable_fg(bg).unwrap_or_default();
                    self.settings.set_on_accent(fg);
                    self.update_hash();
                    self.on_accent.reset_with(fg.to_string());
                    return Action::Render;
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
                } else if modifiers.contains(KeyModifiers::CONTROL)
                    && let KeyCode::Char('g') = key
                {
                    let bg = self.settings.neutral();
                    let fg = Colors::generate_readable_fg(bg).unwrap_or_default();
                    self.settings.set_on_neutral(fg);
                    self.update_hash();
                    self.on_neutral.reset_with(fg.to_string());
                    return Action::Render;
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
    fn new(color: Color) -> Self {
        let mut input = TextInput::from(color.to_string());
        input.move_cursor(crate::widgets::CursorMove::End, false);
        Self(input)
    }

    fn parse_color(&self) -> Result<Color, ratatui::style::ParseColorError> {
        Color::from_str(self.0.as_str().trim())
    }

    const fn set_active(&mut self, active: bool, colors: &Colors) {
        let styles = if active {
            TextInputStyles {
                normal: Style::new(),
                cursor: Style::new().bg(colors.accent).fg(colors.on_accent),
                selector: Style::new().bg(colors.neutral).fg(colors.on_neutral),
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

    fn render(&mut self, line: Rect, buf: &mut Buffer) {
        self.0.render(line, buf);
    }

    fn reset_with(&mut self, s: impl AsRef<str>) {
        self.0.clear();
        self.0.push_str(s.as_ref());
    }
}

fn next(current: usize) -> Option<usize> {
    let mut next = current + 1;
    while next < SETTINGS.len() {
        if SETTINGS[next].filter() {
            return Some(next);
        }
        next += 1;
    }

    None
}

fn previous(current: usize) -> Option<usize> {
    if current == 0 {
        return None;
    }

    let mut prev = current.saturating_sub(1);
    loop {
        if SETTINGS[prev].filter() {
            return Some(prev);
        }

        if prev == 0 {
            break;
        }

        prev -= 1;
    }

    None
}
