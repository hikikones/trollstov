use std::borrow::Cow;

use ratatui::{
    CompletedFrame, DefaultTerminal, Frame,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

/// Default terminal with crossterm backend.
pub struct Terminal(DefaultTerminal);

impl Terminal {
    pub fn init() -> std::io::Result<Self> {
        let terminal = ratatui::try_init()?;
        Ok(Self(terminal))
    }

    pub fn restore(self) -> std::io::Result<()> {
        ratatui::try_restore()
    }

    pub fn draw<F>(&mut self, render_callback: F) -> std::io::Result<CompletedFrame<'_>>
    where
        F: FnOnce(&mut Frame) -> std::io::Result<()>,
    {
        self.0.try_draw(render_callback)
    }

    pub fn temp_leave<T>(&mut self, f: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
        let mut stdout = std::io::stdout();

        execute!(stdout, LeaveAlternateScreen)?;
        disable_raw_mode()?;

        let t = f();

        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen)?;

        self.0.clear()?;

        t
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TerminalColor {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    pub fn query_foreground() -> Result<Self, TerminalQueryError> {
        let Some(fg) = Self::query_osc("\x1b]10;?\x07")? else {
            return Err(TerminalQueryError::Unsupported(
                "terminal does not support OSC 10 color query",
            ));
        };
        Ok(fg)
    }

    pub fn query_background() -> Result<Self, TerminalQueryError> {
        let Some(bg) = Self::query_osc("\x1b]11;?\x07")? else {
            return Err(TerminalQueryError::Unsupported(
                "terminal does not support OSC 11 color query",
            ));
        };
        Ok(bg)
    }

    /// Returns true if the color is perceived as dark.
    pub const fn is_dark(&self) -> bool {
        let brightness = 0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32;
        brightness < 128.0
    }

    pub const fn as_theme(&self) -> TerminalTheme {
        match self.is_dark() {
            true => TerminalTheme::Dark,
            false => TerminalTheme::Light,
        }
    }

    fn query_osc(osc: &str) -> std::io::Result<Option<Self>> {
        let mut buffer = [0; 32];
        let response = query(osc, &mut buffer)?;

        fn parse_osc(response: &str) -> Option<TerminalColor> {
            // Parse response of pattern "\u{1b}]10;rgb:c4c4/c4c4/b5b5\u{1b}\\\u{1b}"
            let start = response.find(':')? + 1;
            let end = response[start..].find('\x1b')?;
            let payload = &response[start..start + end];
            let mut parts = payload.split('/');

            let parse_channel = |s: &str| match s.len() {
                2 => u8::from_str_radix(s, 16).ok(),
                4 => Some((u16::from_str_radix(s, 16).ok()? >> 8) as u8),
                _ => None,
            };

            let r = parse_channel(parts.next()?)?;
            let g = parse_channel(parts.next()?)?;
            let b = parse_channel(parts.next()?)?;

            Some(TerminalColor { r, g, b })
        }

        Ok(parse_osc(&response))
    }

    // pub fn luminance(&self) -> f32 {
    //     fn linearize(c: u8) -> f32 {
    //         let c = c as f32 / 255.0;
    //         if c <= 0.04045 {
    //             c / 12.92
    //         } else {
    //             ((c + 0.055) / 1.055).powf(2.4)
    //         }
    //     }

    //     let r = linearize(self.r);
    //     let g = linearize(self.g);
    //     let b = linearize(self.b);

    //     0.2126 * r + 0.7152 * g + 0.0722 * b
    // }

    // pub fn is_dark(&self) -> bool {
    //     self.luminance() < 0.5
    // }
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalTheme {
    Dark,
    Light,
}

impl TerminalTheme {
    pub fn query() -> Result<Self, TerminalQueryError> {
        let bg = TerminalColor::query_background()?;
        Ok(bg.as_theme())
    }
}

/// The foreground and background colors of the terminal.
#[derive(Debug, Clone, Copy)]
pub struct TerminalPalette {
    pub foreground: TerminalColor,
    pub background: TerminalColor,
}

impl TerminalPalette {
    pub fn query() -> Result<Self, TerminalQueryError> {
        Ok(Self {
            foreground: TerminalColor::query_foreground()?,
            background: TerminalColor::query_background()?,
        })
    }

    pub const fn theme(&self) -> TerminalTheme {
        self.background.as_theme()
    }
}

/// The pixel dimensions of a single cell in the terminal.
#[derive(Debug, Clone, Copy)]
pub struct TerminalCellSize {
    pub width: u32,
    pub height: u32,
}

impl TerminalCellSize {
    pub const DEFAULT: Self = Self {
        width: 10,
        height: 20,
    };

    pub fn query() -> Result<Self, TerminalQueryError> {
        // XTWINOPS protocol, CSI 16 t
        const CELL_SIZE_QUERY: &str = "\x1b[16t";

        let mut buffer = [0; 16];
        let response = query(CELL_SIZE_QUERY, &mut buffer)?;

        // Parse response of pattern "\u{1b}[6;<HEIGHT>;<WIDTH>t\u{1b}[0n"
        let mut split = response.split(";");
        split.next();
        let height = split.next().map(|h| h.parse::<u32>());
        let width = split
            .next()
            .map(|w| w.find('t').map(|i| w[..i].parse::<u32>()))
            .flatten();

        match (height, width) {
            (Some(Ok(height)), Some(Ok(width))) => Ok(Self { width, height }),
            _ => Err(TerminalQueryError::Unsupported(
                "terminal does not support CSI 16 t query for cell size",
            )),
        }
    }
}

impl Default for TerminalCellSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub theme: TerminalTheme,
    pub palette: TerminalPalette,
    pub cell_size: TerminalCellSize,
}

impl TerminalInfo {
    pub fn query() -> Result<Self, TerminalQueryError> {
        let palette = TerminalPalette::query()?;
        Ok(Self {
            theme: palette.theme(),
            palette,
            cell_size: TerminalCellSize::query()?,
        })
    }
}

fn query<'a>(code: &str, buffer: &'a mut [u8]) -> std::io::Result<Cow<'a, str>> {
    use std::io::{Read, Write};

    /// Device Status Report control sequence that most terminals implement.
    /// Makes sure that stdin responds.
    const DEVICE_STATUS_REPORT: &str = "\x1b[5n";

    enable_raw_mode()?;

    // Write query to stdout
    let mut stdout = std::io::stdout();
    stdout.write_fmt(format_args!("{code}{DEVICE_STATUS_REPORT}"))?;
    stdout.flush()?;

    // Read response from stdin
    let n = std::io::stdin().read(buffer)?;

    disable_raw_mode()?;

    let response = String::from_utf8_lossy(&buffer[..n]);
    Ok(response)
}

#[derive(Debug)]
pub enum TerminalQueryError {
    Io(std::io::Error),
    Unsupported(&'static str),
}

impl std::fmt::Display for TerminalQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::Unsupported(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for TerminalQueryError {}

impl From<std::io::Error> for TerminalQueryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
