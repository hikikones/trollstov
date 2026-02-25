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
                neutral: Color::Indexed(242),
                on_neutral: Color::Indexed(252),
            },
            terminal_colorsaurus::ThemeMode::Light => Self {
                accent: Color::Cyan,
                on_accent: Color::Black,
                neutral: Color::Indexed(245),
                on_neutral: Color::Indexed(255),
            },
        }
    }

    pub(super) fn accent(mut self, accent: Option<Color>) -> Self {
        if let Some(accent) = accent {
            self.accent = accent;
            self.on_accent = readable_fg(accent).unwrap_or_default();
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
            self.on_neutral = readable_fg(neutral).unwrap_or_default();
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

fn readable_fg(mut bg: Color) -> Option<Color> {
    if bg == Color::Reset {
        return None;
    }

    if let Color::Indexed(i) = bg {
        bg = indexed_to_color(i);
    }

    let readable_fg = match bg {
        Color::Black => Color::White,
        Color::Red => Color::White,
        Color::Green => Color::Black,
        Color::Yellow => Color::Black,
        Color::Blue => Color::White,
        Color::Magenta => Color::White,
        Color::Cyan => Color::White,
        Color::Gray => Color::DarkGray,
        Color::DarkGray => Color::Gray,
        Color::LightRed => Color::White,
        Color::LightGreen => Color::Black,
        Color::LightYellow => Color::Black,
        Color::LightBlue => Color::Black,
        Color::LightMagenta => Color::White,
        Color::LightCyan => Color::Black,
        Color::White => Color::Black,
        Color::Rgb(r, g, b) => relative_luminance_color(r, g, b),
        Color::Reset | Color::Indexed(_) => {
            unreachable!()
        }
    };

    Some(readable_fg)
}

fn indexed_to_color(i: u8) -> Color {
    match i {
        // 0-15: standard ANSI colors
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,

        // 16-231: 6x6x6 color cube
        16..=231 => {
            let i = i - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;

            let scale = |v| if v == 0 { 0 } else { v * 40 + 55 };

            Color::Rgb(scale(r), scale(g), scale(b))
        }

        // 232-255: grayscale ramp
        232..=255 => {
            let gray = 8 + (i - 232) * 10;
            Color::Rgb(gray, gray, gray)
        }
    }
}

fn relative_luminance_color(r: u8, g: u8, b: u8) -> Color {
    fn srgb_to_linear(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);

    let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    let contrast_black = (l + 0.05) / 0.05;
    let contrast_white = (1.05) / (l + 0.05);

    if contrast_black > contrast_white {
        Color::Black
    } else {
        Color::White
    }
}
