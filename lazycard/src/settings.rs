use std::path::{Path, PathBuf};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use shared::terminal::{TerminalCellSize, TerminalPalette, TerminalTheme};
use widgets::{
    ListColors, MarkupColors, ScrollbarColors, SyntaxHighlightTheme, TextEditorColors,
    TextInputColors, TokenListColors,
};

const VERSION: u8 = 0;

pub struct Settings {
    config: Config,
    path: PathBuf,
    theme: TerminalTheme,
    palette: TerminalPalette,
    cell_size: TerminalCellSize,
}

impl Settings {
    pub const fn new(path: PathBuf, palette: TerminalPalette, cell_size: TerminalCellSize) -> Self {
        let theme = palette.theme();
        Self {
            config: Config::new(theme),
            path,
            theme,
            palette,
            cell_size,
        }
    }

    pub const fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub const fn theme(&self) -> TerminalTheme {
        self.theme
    }

    pub const fn palette(&self) -> TerminalPalette {
        self.palette
    }

    pub const fn cell_size(&self) -> TerminalCellSize {
        self.cell_size
    }

    pub const fn desired_retention(&self) -> u8 {
        self.config.general.desired_retention
    }

    pub const fn desired_retention_as_fraction(&self) -> f32 {
        self.config.general.desired_retention as f32 / 100.0
    }

    pub const fn set_desired_retention(&mut self, percent: u8) {
        self.config.general.desired_retention = if percent > 100 { 100 } else { percent };
    }

    pub const fn colors(&self) -> &Colors {
        &self.config.colors
    }

    pub const fn primary(&self) -> Color {
        self.config.colors.primary
    }

    pub const fn secondary(&self) -> Color {
        self.config.colors.secondary
    }

    pub const fn neutral(&self) -> Color {
        self.config.colors.neutral
    }

    pub const fn set_primary(&mut self, color: Color) {
        self.config.colors.primary = color;
    }

    pub const fn set_secondary(&mut self, color: Color) {
        self.config.colors.secondary = color;
    }

    pub const fn set_neutral(&mut self, color: Color) {
        self.config.colors.neutral = color;
    }

    pub const fn markup_colors(&self) -> MarkupColors {
        MarkupColors {
            syntax_theme: match self.theme {
                TerminalTheme::Dark => SyntaxHighlightTheme::Base16EightiesDark,
                TerminalTheme::Light => SyntaxHighlightTheme::InspiredGitHub,
            },
            scrollbar: self.config.colors.scrollbar(),
            break_char: self.config.colors.neutral,
        }
    }

    pub fn read(
        path: impl AsRef<Path>,
        palette: TerminalPalette,
        cell_size: TerminalCellSize,
    ) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|err| {
            format!(
                "Failed to read settings from '{}' due to {}",
                path.display(),
                err
            )
        })?;

        #[derive(Deserialize)]
        struct V {
            version: u8,
        }

        let V { version } = toml::from_slice(&bytes).map_err(|err| {
            format!(
                "Failed to deserialize settings from '{}' due to {}",
                path.display(),
                err
            )
        })?;

        let config: Config = match version {
            VERSION => toml::from_slice(&bytes).map_err(|err| {
                format!(
                    "Failed to deserialize settings from '{}' due to {}",
                    path.display(),
                    err
                )
            })?,
            _ => Err(format!(
                "Failed to deserialize settings from '{}' due to unknown version",
                path.display()
            ))?,
        };

        Ok(Self {
            config,
            path: path.to_path_buf(),
            theme: palette.theme(),
            palette,
            cell_size,
        })
    }

    pub fn save(&self) -> Result<(), String> {
        let path = self.path.as_path();
        let toml = toml::to_string(&self.config)
            .map_err(|err| format!("Failed to serialize settings due to {}", err))?;
        std::fs::write(path, toml).map_err(|err| {
            format!(
                "Failed to write settings to '{}' due to {}",
                path.display(),
                err
            )
        })?;

        Ok(())
    }

    pub fn hash(&self) -> u64 {
        self.config.hash()
    }

    pub fn as_config(&self) -> Config {
        self.config.clone()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    version: u8,
    general: General,
    colors: Colors,
}

impl Config {
    pub const fn new(theme: TerminalTheme) -> Self {
        Self {
            version: VERSION,
            general: General {
                desired_retention: 80,
            },
            colors: Colors::new(theme),
        }
    }

    pub fn hash(&self) -> u64 {
        toml::to_string(self)
            .map(|s| utils::hash_fast(s))
            .unwrap_or(0)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct General {
    desired_retention: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Colors {
    pub primary: Color,
    pub secondary: Color,
    pub neutral: Color,
}

impl Colors {
    pub const fn new(theme: TerminalTheme) -> Self {
        match theme {
            TerminalTheme::Dark => Self {
                primary: Color::LightYellow,
                secondary: Color::Yellow,
                neutral: Color::Indexed(245),
            },
            TerminalTheme::Light => Self {
                primary: Color::LightCyan,
                secondary: Color::Cyan,
                neutral: Color::Indexed(245),
            },
        }
    }

    pub const fn text_input(&self) -> TextInputColors {
        TextInputColors {
            normal: Color::Reset,
            cursor: self.secondary,
            selector: self.neutral,
            placeholder: self.neutral,
            disabled: self.neutral,
        }
    }

    pub const fn text_editor(&self) -> TextEditorColors {
        TextEditorColors {
            normal: Color::Reset,
            cursor: self.secondary,
            selector: self.neutral,
            placeholder: self.neutral,
            disabled: self.neutral,
        }
    }

    pub const fn list(&self) -> ListColors {
        ListColors {
            scrollbar: self.scrollbar(),
        }
    }

    pub const fn token_list(&self) -> TokenListColors {
        TokenListColors {
            scrollbar: self.scrollbar(),
        }
    }

    pub const fn scrollbar(&self) -> ScrollbarColors {
        ScrollbarColors {
            thumb: self.neutral,
            track: None,
        }
    }
}
