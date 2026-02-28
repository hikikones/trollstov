use std::path::PathBuf;

use jukebox::AudioRating;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "settings.toml";

// TODO: Add version?

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    general: General,
    colors: Colors,
}

impl Default for Settings {
    fn default() -> Self {
        let general = General {
            skip_rating: AudioRating::None,
            keep_selected_track_on_sort: false,
        };
        let colors =
            match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default())
                .unwrap_or(terminal_colorsaurus::ThemeMode::Dark)
            {
                terminal_colorsaurus::ThemeMode::Dark => Colors {
                    accent: Color::Yellow,
                    on_accent: Color::Black,
                    neutral: Color::Indexed(242),
                    on_neutral: Color::Indexed(252),
                },
                terminal_colorsaurus::ThemeMode::Light => Colors {
                    accent: Color::Cyan,
                    on_accent: Color::Black,
                    neutral: Color::Indexed(245),
                    on_neutral: Color::Indexed(255),
                },
            };

        Self { general, colors }
    }
}

impl Settings {
    pub const fn skip_rating(&self) -> AudioRating {
        self.general.skip_rating
    }

    pub const fn keep_on_sort(&self) -> bool {
        self.general.keep_selected_track_on_sort
    }

    pub const fn set_skip_rating(&mut self, rating: AudioRating) {
        self.general.skip_rating = rating;
    }

    pub const fn set_keep_on_sort(&mut self, value: bool) {
        self.general.keep_selected_track_on_sort = value;
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
        let Some(file) = get_config_dir().map(|dir| dir.join(CONFIG_NAME)) else {
            return Ok(Self::default());
        };

        match std::fs::read(&file) {
            Ok(bytes) => {
                let toml: Self = toml::from_slice(&bytes).map_err(|err| {
                    format!(
                        "Failed to deserialize settings from \"{}\" due to {}",
                        file.display(),
                        err
                    )
                })?;
                Ok(toml)
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => Ok(Self::default()),
                _ => Err(format!(
                    "Failed to read settings from \"{}\" due to {}",
                    file.display(),
                    err
                ))?,
            },
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(dir) = get_config_dir() else {
            return Err(
                "Unable to save settings due to no valid home directory path that could be retrieved from the operating system",
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

        let file = dir.join(CONFIG_NAME);
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
    skip_rating: AudioRating,
    keep_selected_track_on_sort: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct Colors {
    accent: Color,
    on_accent: Color,
    neutral: Color,
    on_neutral: Color,
}

fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "hikikones", crate::APP_NAME)
        .map(|project_dirs| project_dirs.config_dir().to_path_buf())
}
