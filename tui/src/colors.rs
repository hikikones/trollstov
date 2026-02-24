use ratatui::style::Color;

pub struct Colors {
    pub accent: Color,
    pub on_accent: Color,
    pub neutral: Color,
    pub on_neutral: Color,
}

impl Colors {
    pub(super) fn new() -> Self {
        match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default())
            .unwrap_or(terminal_colorsaurus::ThemeMode::Dark)
        {
            terminal_colorsaurus::ThemeMode::Dark => Self {
                accent: Color::Yellow,
                on_accent: Color::Black,
                neutral: Color::DarkGray,
                on_neutral: Color::Gray,
            },
            terminal_colorsaurus::ThemeMode::Light => Self {
                accent: Color::Cyan,
                on_accent: Color::Black,
                neutral: Color::DarkGray,
                on_neutral: Color::White,
            },
        }
    }

    pub(super) fn accent(mut self, accent: Option<Color>) -> Self {
        if let Some(accent) = accent {
            self.accent = accent;
        }
        self
    }

    pub(super) fn on_accent(mut self, on_accent: Option<Color>) -> Self {
        if let Some(on_accent) = on_accent {
            self.on_accent = on_accent;
        }
        self
    }

    pub(super) fn neutral(mut self, neutral: Option<Color>) -> Self {
        if let Some(neutral) = neutral {
            self.neutral = neutral;
        }
        self
    }

    pub(super) fn on_neutral(mut self, on_neutral: Option<Color>) -> Self {
        if let Some(on_neutral) = on_neutral {
            self.on_neutral = on_neutral;
        }
        self
    }
}
