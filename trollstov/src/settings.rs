use std::path::PathBuf;

use audio::AudioRating;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

const FILENAME: &str = "settings.toml";
const VERSION: u8 = 0;

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    version: u8,
    general: General,
    colors: Colors,
}

impl Default for Settings {
    fn default() -> Self {
        let general = General {
            skip_tracks_with_rating: AudioRating::None,
            keep_selected_track_on_sort: false,
            search_by_path: false,
        };
        let colors =
            match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default())
                .unwrap_or(terminal_colorsaurus::ThemeMode::Dark)
            {
                terminal_colorsaurus::ThemeMode::Dark => Colors {
                    accent: Color::Yellow,
                    on_accent: Color::Black,
                    neutral: Color::Indexed(245),
                    on_neutral: Color::Black,
                },
                terminal_colorsaurus::ThemeMode::Light => Colors {
                    accent: Color::Cyan,
                    on_accent: Color::Black,
                    neutral: Color::Indexed(245),
                    on_neutral: Color::Black,
                },
            };

        Self {
            version: VERSION,
            general,
            colors,
        }
    }
}

impl Settings {
    pub const fn skip_rating(&self) -> AudioRating {
        self.general.skip_tracks_with_rating
    }

    pub const fn keep_on_sort(&self) -> bool {
        self.general.keep_selected_track_on_sort
    }

    pub const fn search_by_path(&self) -> bool {
        self.general.search_by_path
    }

    pub const fn set_skip_rating(&mut self, rating: AudioRating) {
        self.general.skip_tracks_with_rating = rating;
    }

    pub const fn set_keep_on_sort(&mut self, value: bool) {
        self.general.keep_selected_track_on_sort = value;
    }

    pub const fn set_search_by_path(&mut self, value: bool) {
        self.general.search_by_path = value;
    }

    pub const fn colors(&self) -> &Colors {
        &self.colors
    }

    pub const fn accent(&self) -> Color {
        self.colors.accent
    }

    pub const fn on_accent(&self) -> Color {
        self.colors.on_accent
    }

    pub const fn neutral(&self) -> Color {
        self.colors.neutral
    }

    pub const fn on_neutral(&self) -> Color {
        self.colors.on_neutral
    }

    pub const fn set_accent(&mut self, color: Color) {
        self.colors.accent = color;
    }

    pub const fn set_on_accent(&mut self, color: Color) {
        self.colors.on_accent = color;
    }

    pub const fn set_neutral(&mut self, color: Color) {
        self.colors.neutral = color;
    }

    pub const fn set_on_neutral(&mut self, color: Color) {
        self.colors.on_neutral = color;
    }

    pub fn read() -> Result<Self, Box<dyn std::error::Error>> {
        let Some(file) = get_config_dir().map(|dir| dir.join(FILENAME)) else {
            return Ok(Self::default());
        };

        let bytes = match std::fs::read(&file) {
            Ok(bytes) => bytes,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    return Ok(Self::default());
                }
                _ => {
                    return Err(format!(
                        "Failed to read settings from \"{}\" due to {}",
                        file.display(),
                        err
                    ))?;
                }
            },
        };

        #[derive(Deserialize)]
        struct V {
            version: u8,
        }

        let V { version } = toml::from_slice(&bytes).map_err(|err| {
            format!(
                "Failed to deserialize settings from \"{}\" due to {}",
                file.display(),
                err
            )
        })?;

        match version {
            VERSION => {
                let settings: Self = toml::from_slice(&bytes).map_err(|err| {
                    format!(
                        "Failed to deserialize settings from \"{}\" due to {}",
                        file.display(),
                        err
                    )
                })?;
                Ok(settings)
            }
            _ => Err(format!(
                "Failed to deserialize settings from \"{}\" due to unknown version",
                file.display()
            ))?,
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(dir) = get_config_dir() else {
            return Err(
                "Unable to save settings due to no valid home directory path \
                that could be retrieved from the operating system",
            )?;
        };

        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|err| {
                format!(
                    "Failed to create directory \"{}\" due to {}",
                    dir.display(),
                    err
                )
            })?;
        }

        let file = dir.join(FILENAME);
        let toml = toml::to_string(self)
            .map_err(|err| format!("Failed to serialize settings due to {}", err))?;
        std::fs::write(file, toml).map_err(|err| {
            format!(
                "Failed to write settings to \"{}\" due to {}",
                dir.display(),
                err
            )
        })?;

        Ok(())
    }

    pub fn hash(&self) -> u64 {
        toml::to_string(self)
            .map(|s| seahash::hash(s.as_bytes()))
            .unwrap_or(0)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct General {
    skip_tracks_with_rating: AudioRating,
    keep_selected_track_on_sort: bool,
    search_by_path: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Colors {
    pub accent: Color,
    pub on_accent: Color,
    pub neutral: Color,
    pub on_neutral: Color,
}

impl Colors {
    pub fn generate_readable_fg(bg: Color) -> Option<Color> {
        readable_fg(bg)
    }
}

fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "hikikones", crate::APP_NAME)
        .map(|project_dirs| project_dirs.config_dir().to_path_buf())
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
