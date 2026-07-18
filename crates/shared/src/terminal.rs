// The foreground and background colors of the terminal.
#[derive(Debug, Clone, Copy)]
pub struct TerminalPalette {
    pub foreground: TerminalColor,
    pub background: TerminalColor,
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TerminalPalette {
    pub fn query() -> Result<Self, TerminalQueryError> {
        // OSC color queries
        const FG_OSC_QUERY: &str = "\x1b]10;?\x07";
        const BG_OSC_QUERY: &str = "\x1b]11;?\x07";

        let Some(fg) = query_osc_color(FG_OSC_QUERY)? else {
            return Err(TerminalQueryError::Unsupported(
                "terminal does not support OSC 10 color query",
            ));
        };
        let Some(bg) = query_osc_color(BG_OSC_QUERY)? else {
            return Err(TerminalQueryError::Unsupported(
                "terminal does not support OSC 11 color query",
            ));
        };

        fn query_osc_color(osc: &str) -> std::io::Result<Option<TerminalColor>> {
            let mut buffer = [0; 32];
            let response = query(osc, &mut buffer)?;
            dbg!(&response);

            fn parse_osc_color(response: &str) -> Option<TerminalColor> {
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

            Ok(parse_osc_color(&response))
        }

        Ok(Self {
            foreground: fg,
            background: bg,
        })
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

fn query<'a>(code: &str, buffer: &'a mut [u8]) -> std::io::Result<std::borrow::Cow<'a, str>> {
    use std::io::{Read, Write};

    /// Device Status Report control sequence that most terminals implement.
    /// Makes sure that stdin responds.
    const DEVICE_STATUS_REPORT: &str = "\x1b[5n";

    ratatui::crossterm::terminal::enable_raw_mode()?;

    // Write query to stdout
    let mut stdout = std::io::stdout();
    stdout.write_fmt(format_args!("{code}{DEVICE_STATUS_REPORT}"))?;
    stdout.flush()?;

    // Read response from stdin
    let n = std::io::stdin().read(buffer)?;

    ratatui::crossterm::terminal::disable_raw_mode()?;

    let reponse = String::from_utf8_lossy(&buffer[..n]);
    Ok(reponse)
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
