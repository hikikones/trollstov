use std::str::FromStr;

use audio::AudioRating;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use widgets::{
    CursorMove, List, ListItem, ListMove, Shortcut, Shortcuts, TextInput, TextInputStyles,
    TextSegment,
};

use crate::{
    app::Action,
    pages::Log,
    settings::{Colors, Settings},
    symbols,
};

pub struct SettingsPage {
    settings: Settings,
    applied: Settings,
    written: Settings,
    default: Settings,
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
    KeepTrackSort,
    SearchByPath,
    Colors,
    AccentColor,
    OnAccentColor,
    NeutralColor,
    OnNeutralColor,
    Empty,
}

impl Setting {
    const fn filter(&self) -> bool {
        match self {
            Self::General => false,
            Self::SkipRating => true,
            Self::KeepTrackSort => true,
            Self::SearchByPath => true,
            Self::Colors => false,
            Self::AccentColor => true,
            Self::OnAccentColor => true,
            Self::NeutralColor => true,
            Self::OnNeutralColor => true,
            Self::Empty => false,
        }
    }
}

const SETTINGS: [Setting; 12] = [
    Setting::General,
    Setting::Empty,
    Setting::SkipRating,
    Setting::KeepTrackSort,
    Setting::SearchByPath,
    Setting::Empty,
    Setting::Colors,
    Setting::Empty,
    Setting::AccentColor,
    Setting::OnAccentColor,
    Setting::NeutralColor,
    Setting::OnNeutralColor,
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
            default: Settings::default(),
            apply_hash: hash,
            write_hash: hash,
            is_applied: true,
            is_written: true,
            list: List::new().with_index(selected).with_margins(3, 3),
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
            .padding(Padding::uniform(1));
        let settings_area = block.inner(area);
        block.render(area, buf);

        let description_area = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..settings_area
        };

        // General areas
        let mut general_area = Rect {
            width: settings_area.width / 2,
            ..settings_area
        };
        let mut general_input_area = Rect {
            x: general_area.x + general_area.width + 1,
            width: general_area.width.saturating_sub(1),
            ..general_area
        };

        // Color areas
        let [
            mut color_area,
            _,
            mut color_input_area,
            _,
            mut color_preview_area,
        ] = Layout::horizontal([
            Constraint::Percentage(42),
            Constraint::Length(1),
            Constraint::Max(8),
            Constraint::Length(2),
            Constraint::Fill(0),
        ])
        .areas(settings_area);

        let colors = settings.colors();
        let current_setting = self.current();

        self.list.set_colors(colors.neutral, None).render(
            settings_area,
            buf,
            SETTINGS,
            |line, buf, setting, index| {
                general_area.y = line.y;
                general_input_area.y = line.y;
                color_area.y = line.y;
                color_input_area.y = line.y;
                color_preview_area.y = line.y;

                let (symbol, style) = if index == ListItem::Selected {
                    (
                        symbols::concat!(symbols::SELECTED, " "),
                        Style::new().bold(),
                    )
                } else {
                    ("", Style::new())
                };

                match setting {
                    Setting::General => {
                        print_section(line, buf, "GENERAL");
                    }
                    Setting::SkipRating => {
                        print_rating(
                            general_area,
                            general_input_area,
                            buf,
                            symbol,
                            "Skip tracks with rating",
                            style,
                            self.settings.skip_rating(),
                            colors,
                        );
                    }
                    Setting::KeepTrackSort => {
                        print_checkmark(
                            general_area,
                            general_input_area,
                            buf,
                            symbol,
                            "Keep selected track on sort",
                            style,
                            self.settings.keep_on_sort(),
                            colors,
                        );
                    }
                    Setting::SearchByPath => {
                        print_checkmark(
                            general_area,
                            general_input_area,
                            buf,
                            symbol,
                            "Search by path",
                            style,
                            self.settings.search_by_path(),
                            colors,
                        );
                    }
                    Setting::Colors => {
                        print_section(line, buf, "COLORS");
                    }
                    Setting::AccentColor => {
                        print_color(
                            color_area,
                            color_input_area,
                            color_preview_area,
                            buf,
                            symbol,
                            "Set accent color",
                            style,
                            &mut self.accent,
                            current_setting == Setting::AccentColor,
                            colors,
                            "accent color",
                            Style::new().fg(self.settings.accent()),
                        );
                    }
                    Setting::OnAccentColor => {
                        print_color(
                            color_area,
                            color_input_area,
                            color_preview_area,
                            buf,
                            symbol,
                            "Set on accent color",
                            style,
                            &mut self.on_accent,
                            current_setting == Setting::OnAccentColor,
                            colors,
                            "on accent color",
                            Style::new()
                                .bg(self.settings.accent())
                                .fg(self.settings.on_accent()),
                        );
                    }
                    Setting::NeutralColor => {
                        print_color(
                            color_area,
                            color_input_area,
                            color_preview_area,
                            buf,
                            symbol,
                            "Set neutral color",
                            style,
                            &mut self.neutral,
                            current_setting == Setting::NeutralColor,
                            colors,
                            "neutral color",
                            Style::new().fg(self.settings.neutral()),
                        );
                    }
                    Setting::OnNeutralColor => {
                        print_color(
                            color_area,
                            color_input_area,
                            color_preview_area,
                            buf,
                            symbol,
                            "Set on neutral color",
                            style,
                            &mut self.on_neutral,
                            current_setting == Setting::OnNeutralColor,
                            colors,
                            "on neutral color",
                            Style::new()
                                .bg(self.settings.neutral())
                                .fg(self.settings.on_neutral()),
                        );
                    }
                    Setting::Empty => {}
                }

                self.text.clear();
            },
        );

        // Description and shortcuts
        const COLOR_DESCRIPTION: &str = "Set color by name, hex code or indexed value";
        let description = match current_setting {
            Setting::SkipRating => {
                shortcuts.push(Shortcut::new("Rating", "0-5"));
                "Skips tracks that are less than or equal to set rating"
            }
            Setting::KeepTrackSort => {
                shortcuts.push(Shortcut::new("Toggle", symbols::SPACE));
                "Scrolls to selected track when sorting"
            }
            Setting::SearchByPath => {
                shortcuts.push(Shortcut::new("Toggle", symbols::SPACE));
                "Includes directories and filename when searching"
            }
            Setting::AccentColor | Setting::NeutralColor => {
                shortcuts.push(Shortcut::new("Set color", symbols::ENTER));
                COLOR_DESCRIPTION
            }
            Setting::OnAccentColor | Setting::OnNeutralColor => {
                shortcuts.extend([
                    Shortcut::new("Set color", symbols::ENTER),
                    Shortcut::new("Generate color", symbols::ctrl!("g")),
                ]);
                COLOR_DESCRIPTION
            }
            Setting::General | Setting::Colors | Setting::Empty => "",
        };

        if !description.is_empty() {
            widgets::print_asciis(
                description_area,
                buf,
                [" ", description, " "],
                Style::new(),
                Some(widgets::Alignment::CenterHorizontal),
            );
        }

        if !self.is_applied {
            shortcuts.push(Shortcut::new("Apply", symbols::ctrl!("a")));
        }
        if !self.is_written {
            shortcuts.push(Shortcut::new("Save", symbols::ctrl!("s")));
        }

        // Always show reset all
        shortcuts.push(Shortcut::new("Reset all", symbols::ctrl!("r")));
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
                'r' => {
                    if ctrl {
                        self.settings = self.default.clone();
                        self.accent.reset_with(self.settings.accent());
                        self.on_accent.reset_with(self.settings.on_accent());
                        self.neutral.reset_with(self.settings.neutral());
                        self.on_neutral.reset_with(self.settings.on_neutral());
                        self.update_hash();
                        return Action::Render;
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
            Setting::SearchByPath => {
                if let KeyCode::Char(' ') = key {
                    let toggle = !self.settings.search_by_path();
                    self.settings.set_search_by_path(toggle);
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
                    self.on_accent.reset_with(fg);
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
                    self.on_neutral.reset_with(fg);
                    return Action::Render;
                } else if self.on_neutral.input(key, modifiers) {
                    return Action::Render;
                }
            }
            Setting::General | Setting::Colors | Setting::Empty => {}
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
        input.move_cursor(CursorMove::End, false);
        Self(input)
    }

    fn parse_color(&self) -> Result<Color, String> {
        let input = self.0.as_str_trim();
        Color::from_str(input).map_err(|_| format!("Failed to parse \"{input}\" as a color"))
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

    fn reset_with(&mut self, color: Color) {
        self.0.clear();
        self.0.push_str(color.to_string().as_str());
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

fn print_section(line: Rect, buf: &mut Buffer, ascii: &str) {
    widgets::print_ascii(
        line,
        buf,
        ascii,
        Style::new(),
        Some(widgets::Alignment::CenterHorizontal),
    );
}

fn print_rating(
    text_area: Rect,
    input_area: Rect,
    buf: &mut Buffer,
    symbol: &str,
    text: &str,
    style: Style,
    rating: AudioRating,
    colors: &Colors,
) {
    // Text
    widgets::print_asciis(
        text_area,
        buf,
        [symbol, text, ":"],
        style,
        Some(widgets::Alignment::Right),
    );

    // Stars
    let stars = symbols::stars_split(rating);
    widgets::print_texts_with_styles(
        input_area,
        buf,
        [
            (stars.0, Style::new().fg(colors.accent)),
            (stars.1, Style::new().fg(colors.neutral)),
        ],
        None,
        None,
    );
}

fn print_checkmark(
    text_area: Rect,
    input_area: Rect,
    buf: &mut Buffer,
    symbol: &str,
    text: &str,
    style: Style,
    checkmark: bool,
    colors: &Colors,
) {
    // Text
    widgets::print_asciis(
        text_area,
        buf,
        [symbol, text, ":"],
        style,
        Some(widgets::Alignment::Right),
    );

    // Checkmark
    let (checkmark, color) = match checkmark {
        true => (symbols::CHECKMARK_YES, colors.accent),
        false => (symbols::CHECKMARK_NO, colors.neutral),
    };
    let Rect { x, y, .. } = input_area;
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(checkmark).set_style(color);
    }
}

fn print_color(
    text_area: Rect,
    input_area: Rect,
    preview_area: Rect,
    buf: &mut Buffer,
    symbol: &str,
    text: &str,
    style: Style,
    color_setting: &mut ColorSetting,
    color_is_active: bool,
    colors: &Colors,
    preview_text: &str,
    preview_style: Style,
) {
    // Text
    widgets::print_asciis(
        text_area,
        buf,
        [symbol, text, ":"],
        style,
        Some(widgets::Alignment::Right),
    );

    // Input
    color_setting.set_active(color_is_active, colors);
    color_setting.render(input_area, buf);

    // Preview
    widgets::print_ascii(preview_area, buf, preview_text, preview_style, None);
}
