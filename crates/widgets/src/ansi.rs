/// ANSI writer and parser for most Select Graphic Rendition (SGR) attributes.
use std::{fmt::Write, ops::Range, str::CharIndices};

use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{HorizontalAlignment, Rect, Size},
    style::{Color, Modifier, Style},
    widgets::Padding,
};

use crate::{
    RectExt, ScrollMove, ScrollableData, Scrollbar, ScrollbarColors, ScrollbarData, TextSpan,
};

const ANSI_START: char = '\x1b';
const ANSI_START2: char = '[';
const ANSI_END: char = 'm';

#[derive(Debug, Clone, Copy)]
pub enum AnsiTag {
    // Reset
    Reset,

    // Style on
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Conceal,
    CrossedOut,
    // Framed,
    // Encircled,
    // Overlined,

    // Style off
    NotBold,
    NotItalic,
    NotUnderline,
    NotBlink,
    NotReverse,
    Reveal,
    NotCrossedOut,
    // NotFramedOrEncircled,
    // NotOverlined,

    // Foreground colors
    FgBlack,
    FgRed,
    FgGreen,
    FgYellow,
    FgBlue,
    FgMagenta,
    FgCyan,
    FgWhite,
    FgBrightBlack,
    FgBrightRed,
    FgBrightGreen,
    FgBrightYellow,
    FgBrightBlue,
    FgBrightMagenta,
    FgBrightCyan,
    FgBrightWhite,
    FgDefault,

    // Background colors
    BgBlack,
    BgRed,
    BgGreen,
    BgYellow,
    BgBlue,
    BgMagenta,
    BgCyan,
    BgWhite,
    BgBrightBlack,
    BgBrightRed,
    BgBrightGreen,
    BgBrightYellow,
    BgBrightBlue,
    BgBrightMagenta,
    BgBrightCyan,
    BgBrightWhite,
    BgDefault,

    // Extended colors
    Fg256(u8),
    Bg256(u8),
    FgTrueColor(u8, u8, u8),
    BgTrueColor(u8, u8, u8),
}

impl AnsiTag {
    const fn from_u8(value: u8) -> Option<Self> {
        let tag = match value {
            0 => Self::Reset,

            1 => Self::Bold,
            2 => Self::Faint,
            3 => Self::Italic,
            4 => Self::Underline,
            5 => Self::SlowBlink,
            6 => Self::RapidBlink,
            7 => Self::Reverse,
            8 => Self::Conceal,
            9 => Self::CrossedOut,

            22 => Self::NotBold,
            23 => Self::NotItalic,
            24 => Self::NotUnderline,
            25 => Self::NotBlink,
            27 => Self::NotReverse,
            28 => Self::Reveal,
            29 => Self::NotCrossedOut,

            30 => Self::FgBlack,
            31 => Self::FgRed,
            32 => Self::FgGreen,
            33 => Self::FgYellow,
            34 => Self::FgBlue,
            35 => Self::FgMagenta,
            36 => Self::FgCyan,
            37 => Self::FgWhite,

            39 => Self::FgDefault,

            40 => Self::BgBlack,
            41 => Self::BgRed,
            42 => Self::BgGreen,
            43 => Self::BgYellow,
            44 => Self::BgBlue,
            45 => Self::BgMagenta,
            46 => Self::BgCyan,
            47 => Self::BgWhite,

            49 => Self::BgDefault,

            // 51 => Self::Framed,
            // 52 => Self::Encircled,
            // 53 => Self::Overlined,
            // 54 => Self::NotFramedOrEncircled,
            // 55 => Self::NotOverlined,

            //
            90 => Self::FgBrightBlack,
            91 => Self::FgBrightRed,
            92 => Self::FgBrightGreen,
            93 => Self::FgBrightYellow,
            94 => Self::FgBrightBlue,
            95 => Self::FgBrightMagenta,
            96 => Self::FgBrightCyan,
            97 => Self::FgBrightWhite,

            100 => Self::BgBrightBlack,
            101 => Self::BgBrightRed,
            102 => Self::BgBrightGreen,
            103 => Self::BgBrightYellow,
            104 => Self::BgBrightBlue,
            105 => Self::BgBrightMagenta,
            106 => Self::BgBrightCyan,
            107 => Self::BgBrightWhite,

            _ => return None,
        };

        Some(tag)
    }

    pub const fn from_color_fg(color: Color) -> Self {
        match color {
            Color::Reset => Self::FgDefault,
            Color::Black => Self::FgBlack,
            Color::Red => Self::FgRed,
            Color::Green => Self::FgGreen,
            Color::Yellow => Self::FgYellow,
            Color::Blue => Self::FgBlue,
            Color::Magenta => Self::FgMagenta,
            Color::Cyan => Self::FgCyan,
            Color::Gray => Self::FgWhite,
            Color::DarkGray => Self::FgBrightBlack,
            Color::LightRed => Self::FgBrightRed,
            Color::LightGreen => Self::FgBrightGreen,
            Color::LightYellow => Self::FgBrightYellow,
            Color::LightBlue => Self::FgBrightBlue,
            Color::LightMagenta => Self::FgBrightMagenta,
            Color::LightCyan => Self::FgBrightCyan,
            Color::White => Self::FgBrightWhite,
            Color::Rgb(r, g, b) => Self::FgTrueColor(r, g, b),
            Color::Indexed(n) => Self::Fg256(n),
        }
    }

    pub const fn from_color_bg(color: Color) -> Self {
        match color {
            Color::Reset => Self::BgDefault,
            Color::Black => Self::BgBlack,
            Color::Red => Self::BgRed,
            Color::Green => Self::BgGreen,
            Color::Yellow => Self::BgYellow,
            Color::Blue => Self::BgBlue,
            Color::Magenta => Self::BgMagenta,
            Color::Cyan => Self::BgCyan,
            Color::Gray => Self::BgWhite,
            Color::DarkGray => Self::BgBrightBlack,
            Color::LightRed => Self::BgBrightRed,
            Color::LightGreen => Self::BgBrightGreen,
            Color::LightYellow => Self::BgBrightYellow,
            Color::LightBlue => Self::BgBrightBlue,
            Color::LightMagenta => Self::BgBrightMagenta,
            Color::LightCyan => Self::BgBrightCyan,
            Color::White => Self::BgBrightWhite,
            Color::Rgb(r, g, b) => Self::BgTrueColor(r, g, b),
            Color::Indexed(n) => Self::Bg256(n),
        }
    }

    pub fn as_style(self) -> Style {
        let mut style = Style::new();
        self.apply_to_style(&mut style);
        style
    }

    pub fn apply_to_style(self, style: &mut Style) {
        match self {
            AnsiTag::Reset => {
                *style = Style::new();
            }
            AnsiTag::Bold => {
                style.add_modifier.insert(Modifier::BOLD);
            }
            AnsiTag::Faint => {
                style.add_modifier.insert(Modifier::DIM);
            }
            AnsiTag::Italic => {
                style.add_modifier.insert(Modifier::ITALIC);
            }
            AnsiTag::Underline => {
                style.add_modifier.insert(Modifier::UNDERLINED);
            }
            AnsiTag::SlowBlink => {
                style.add_modifier.insert(Modifier::SLOW_BLINK);
            }
            AnsiTag::RapidBlink => {
                style.add_modifier.insert(Modifier::RAPID_BLINK);
            }
            AnsiTag::Reverse => {
                style.add_modifier.insert(Modifier::REVERSED);
            }
            AnsiTag::Conceal => {
                style.add_modifier.insert(Modifier::HIDDEN);
            }
            AnsiTag::CrossedOut => {
                style.add_modifier.insert(Modifier::CROSSED_OUT);
            }
            // AnsiTag::Framed => todo!(),
            // AnsiTag::Encircled => todo!(),
            // AnsiTag::Overlined => todo!(),
            AnsiTag::NotBold => {
                style.add_modifier.remove(Modifier::BOLD);
            }
            AnsiTag::NotItalic => {
                style.add_modifier.remove(Modifier::ITALIC);
            }
            AnsiTag::NotUnderline => {
                style.add_modifier.remove(Modifier::UNDERLINED);
            }
            AnsiTag::NotBlink => {
                style.add_modifier.remove(Modifier::SLOW_BLINK);
                style.add_modifier.remove(Modifier::RAPID_BLINK);
            }
            AnsiTag::NotReverse => {
                style.add_modifier.remove(Modifier::REVERSED);
            }
            AnsiTag::Reveal => {
                style.add_modifier.remove(Modifier::HIDDEN);
            }
            AnsiTag::NotCrossedOut => {
                style.add_modifier.remove(Modifier::CROSSED_OUT);
            }
            // AnsiTag::NotFramedOrEncircled => todo!(),
            // AnsiTag::NotOverlined => todo!(),
            AnsiTag::FgBlack => {
                style.fg = Some(Color::Black);
            }
            AnsiTag::FgRed => {
                style.fg = Some(Color::Red);
            }
            AnsiTag::FgGreen => {
                style.fg = Some(Color::Green);
            }
            AnsiTag::FgYellow => {
                style.fg = Some(Color::Yellow);
            }
            AnsiTag::FgBlue => {
                style.fg = Some(Color::Blue);
            }
            AnsiTag::FgMagenta => {
                style.fg = Some(Color::Magenta);
            }
            AnsiTag::FgCyan => {
                style.fg = Some(Color::Cyan);
            }
            AnsiTag::FgWhite => {
                style.fg = Some(Color::Gray);
            }
            AnsiTag::FgBrightBlack => {
                style.fg = Some(Color::DarkGray);
            }
            AnsiTag::FgBrightRed => {
                style.fg = Some(Color::LightRed);
            }
            AnsiTag::FgBrightGreen => {
                style.fg = Some(Color::LightGreen);
            }
            AnsiTag::FgBrightYellow => {
                style.fg = Some(Color::LightYellow);
            }
            AnsiTag::FgBrightBlue => {
                style.fg = Some(Color::LightBlue);
            }
            AnsiTag::FgBrightMagenta => {
                style.fg = Some(Color::LightMagenta);
            }
            AnsiTag::FgBrightCyan => {
                style.fg = Some(Color::LightCyan);
            }
            AnsiTag::FgBrightWhite => {
                style.fg = Some(Color::White);
            }
            AnsiTag::FgDefault => {
                style.fg = Some(Color::Reset);
            }
            AnsiTag::BgBlack => {
                style.bg = Some(Color::Black);
            }
            AnsiTag::BgRed => {
                style.bg = Some(Color::Red);
            }
            AnsiTag::BgGreen => {
                style.bg = Some(Color::Green);
            }
            AnsiTag::BgYellow => {
                style.bg = Some(Color::Yellow);
            }
            AnsiTag::BgBlue => {
                style.bg = Some(Color::Blue);
            }
            AnsiTag::BgMagenta => {
                style.bg = Some(Color::Magenta);
            }
            AnsiTag::BgCyan => {
                style.bg = Some(Color::Cyan);
            }
            AnsiTag::BgWhite => {
                style.bg = Some(Color::Gray);
            }
            AnsiTag::BgBrightBlack => {
                style.bg = Some(Color::DarkGray);
            }
            AnsiTag::BgBrightRed => {
                style.bg = Some(Color::LightRed);
            }
            AnsiTag::BgBrightGreen => {
                style.bg = Some(Color::LightGreen);
            }
            AnsiTag::BgBrightYellow => {
                style.bg = Some(Color::LightYellow);
            }
            AnsiTag::BgBrightBlue => {
                style.bg = Some(Color::LightBlue);
            }
            AnsiTag::BgBrightMagenta => {
                style.bg = Some(Color::LightMagenta);
            }
            AnsiTag::BgBrightCyan => {
                style.bg = Some(Color::LightCyan);
            }
            AnsiTag::BgBrightWhite => {
                style.bg = Some(Color::White);
            }
            AnsiTag::BgDefault => {
                style.bg = Some(Color::Reset);
            }
            AnsiTag::Fg256(i) => {
                style.fg = Some(Color::Indexed(i));
            }
            AnsiTag::Bg256(i) => {
                style.bg = Some(Color::Indexed(i));
            }
            AnsiTag::FgTrueColor(r, g, b) => {
                style.fg = Some(Color::Rgb(r, g, b));
            }
            AnsiTag::BgTrueColor(r, g, b) => {
                style.bg = Some(Color::Rgb(r, g, b));
            }
        }
    }
}

impl std::fmt::Display for AnsiTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(ANSI_START)?;
        f.write_char(ANSI_START2)?;

        match *self {
            Self::Reset => f.write_char('0')?,

            Self::Bold => f.write_char('1')?,
            Self::Faint => f.write_char('2')?,
            Self::Italic => f.write_char('3')?,
            Self::Underline => f.write_char('4')?,
            Self::SlowBlink => f.write_char('5')?,
            Self::RapidBlink => f.write_char('6')?,
            Self::Reverse => f.write_char('7')?,
            Self::Conceal => f.write_char('8')?,
            Self::CrossedOut => f.write_char('9')?,
            // Self::Framed => f.write_str("51")?,
            // Self::Encircled => f.write_str("52")?,
            // Self::Overlined => f.write_str("53")?,

            //
            Self::NotBold => f.write_str("22")?,
            Self::NotItalic => f.write_str("23")?,
            Self::NotUnderline => f.write_str("24")?,
            Self::NotBlink => f.write_str("25")?,
            Self::NotReverse => f.write_str("27")?,
            Self::Reveal => f.write_str("28")?,
            Self::NotCrossedOut => f.write_str("29")?,
            // Self::NotFramedOrEncircled => f.write_str("54")?,
            // Self::NotOverlined => f.write_str("55")?,

            //
            Self::FgBlack => f.write_str("30")?,
            Self::FgRed => f.write_str("31")?,
            Self::FgGreen => f.write_str("32")?,
            Self::FgYellow => f.write_str("33")?,
            Self::FgBlue => f.write_str("34")?,
            Self::FgMagenta => f.write_str("35")?,
            Self::FgCyan => f.write_str("36")?,
            Self::FgWhite => f.write_str("37")?,

            Self::FgBrightBlack => f.write_str("90")?,
            Self::FgBrightRed => f.write_str("91")?,
            Self::FgBrightGreen => f.write_str("92")?,
            Self::FgBrightYellow => f.write_str("93")?,
            Self::FgBrightBlue => f.write_str("94")?,
            Self::FgBrightMagenta => f.write_str("95")?,
            Self::FgBrightCyan => f.write_str("96")?,
            Self::FgBrightWhite => f.write_str("97")?,

            Self::FgDefault => f.write_str("39")?,

            Self::BgBlack => f.write_str("40")?,
            Self::BgRed => f.write_str("41")?,
            Self::BgGreen => f.write_str("42")?,
            Self::BgYellow => f.write_str("43")?,
            Self::BgBlue => f.write_str("44")?,
            Self::BgMagenta => f.write_str("45")?,
            Self::BgCyan => f.write_str("46")?,
            Self::BgWhite => f.write_str("47")?,

            Self::BgBrightBlack => f.write_str("100")?,
            Self::BgBrightRed => f.write_str("101")?,
            Self::BgBrightGreen => f.write_str("102")?,
            Self::BgBrightYellow => f.write_str("103")?,
            Self::BgBrightBlue => f.write_str("104")?,
            Self::BgBrightMagenta => f.write_str("105")?,
            Self::BgBrightCyan => f.write_str("106")?,
            Self::BgBrightWhite => f.write_str("107")?,

            Self::BgDefault => f.write_str("49")?,

            Self::Fg256(n) => f.write_fmt(format_args!("38;5;{}", n))?,
            Self::Bg256(n) => f.write_fmt(format_args!("48;5;{}", n))?,
            Self::FgTrueColor(r, g, b) => f.write_fmt(format_args!("38;2;{};{};{}", r, g, b))?,
            Self::BgTrueColor(r, g, b) => f.write_fmt(format_args!("48;2;{};{};{}", r, g, b))?,
        }

        f.write_char(ANSI_END)
    }
}

pub struct AnsiWriter {
    inner: String,
}

impl AnsiWriter {
    pub const fn new() -> Self {
        Self {
            inner: String::new(),
        }
    }

    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    pub const fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub const fn inner(&self) -> &String {
        &self.inner
    }

    pub const fn inner_mut(&mut self) -> &mut String {
        &mut self.inner
    }

    pub fn slice(&self, range: Range<usize>) -> &str {
        &self.inner[range]
    }

    pub fn push_char(&mut self, c: char) {
        self.inner.push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        self.inner.push_str(s);
    }

    pub fn push_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        use std::fmt::Write;
        let _ = self.inner.write_fmt(args);
    }

    pub fn push_tag(&mut self, tag: AnsiTag) {
        let _ = self.inner.write_fmt(format_args!("{tag}"));
    }

    pub fn insert_char(&mut self, i: usize, ch: char) {
        self.inner.insert(i, ch);
    }

    pub fn insert_str(&mut self, i: usize, s: &str) {
        self.inner.insert_str(i, s);
    }

    pub fn insert_tag(&mut self, i: usize, tag: AnsiTag) {
        // TODO: use compact string lib?
        self.inner.insert_str(i, &format!("{tag}"));
    }

    pub fn extend<'a>(&mut self, iter: impl IntoIterator<Item = &'a str>) {
        self.inner.extend(iter);
    }

    pub fn textwrap(&mut self, width: u16) {
        textwrap::fill_inplace(&mut self.inner, width as usize);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

#[derive(Debug)]
pub enum AnsiEvent<'a> {
    Text(&'a str),
    Tag(AnsiTag),
}

pub struct AnsiParser<'a> {
    input: &'a str,
    chars: CharIndices<'a>,
    start: usize,
    tag: Option<AnsiTag>,
}

impl<'a> AnsiParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices(),
            start: 0,
            tag: None,
        }
    }

    pub const fn with_style(self) -> AnsiParserWithStyle<'a> {
        AnsiParserWithStyle {
            parser: self,
            style: Style::new(),
        }
    }

    fn parse_ansi_code(&mut self) -> Option<(usize, AnsiTag)> {
        let Some((i, ANSI_START2)) = self.chars.next() else {
            return None;
        };

        let code_start = i + ANSI_START2.len_utf8();
        let Some(code_end) = self.find_ansi_end() else {
            return None;
        };

        let code = &self.input[code_start..code_end];
        let tag = if code.contains(";") {
            // Extended colors
            let mut split = code.split(";");
            let count = split.clone().count();
            match count {
                3 => {
                    // Indexed color
                    match (
                        split.next(),
                        split.next(),
                        split.next().map(|n| n.parse::<u8>()),
                    ) {
                        (Some("38"), Some("5"), Some(Ok(index))) => Some(AnsiTag::Fg256(index)),
                        (Some("48"), Some("5"), Some(Ok(index))) => Some(AnsiTag::Bg256(index)),
                        _ => return None,
                    }
                }
                5 => {
                    // True color
                    match (
                        split.next(),
                        split.next(),
                        split.next().map(|n| n.parse::<u8>()),
                        split.next().map(|n| n.parse::<u8>()),
                        split.next().map(|n| n.parse::<u8>()),
                    ) {
                        (Some("38"), Some("2"), Some(Ok(r)), Some(Ok(g)), Some(Ok(b))) => {
                            Some(AnsiTag::FgTrueColor(r, g, b))
                        }
                        (Some("48"), Some("2"), Some(Ok(r)), Some(Ok(g)), Some(Ok(b))) => {
                            Some(AnsiTag::BgTrueColor(r, g, b))
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        } else {
            // Single code
            let Ok(num) = code.parse::<u8>() else {
                return None;
            };
            AnsiTag::from_u8(num)
        };

        tag.map(|tag| (code_end + ANSI_END.len_utf8(), tag))
    }

    fn find_ansi_end(&mut self) -> Option<usize> {
        let mut end = None;
        let max_code_len = 17; // Should be enough for code len
        for _ in 0..max_code_len {
            match self.chars.next() {
                Some((i, c)) => {
                    if c == ANSI_END {
                        end = Some(i);
                        break;
                    }
                }
                None => break,
            }
        }
        end
    }
}

impl<'a> Iterator for AnsiParser<'a> {
    type Item = AnsiEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tag) = self.tag.take() {
            return Some(AnsiEvent::Tag(tag));
        }

        while let Some((i, c)) = self.chars.next() {
            if c == ANSI_START
                && let Some((end, tag)) = self.parse_ansi_code()
            {
                let event = if self.start < i {
                    // Set tag and return text
                    self.tag = Some(tag);
                    AnsiEvent::Text(&self.input[self.start..i])
                } else {
                    // Return tag
                    AnsiEvent::Tag(tag)
                };
                self.start = end;
                return Some(event);
            }
        }

        let remaining = &self.input[self.start..];
        self.start = self.input.len();

        if remaining.is_empty() {
            None
        } else {
            Some(AnsiEvent::Text(remaining))
        }
    }
}

pub struct AnsiParserWithStyle<'a> {
    parser: AnsiParser<'a>,
    style: Style,
}

impl<'a> AnsiParserWithStyle<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            parser: AnsiParser::new(input),
            style: Style::new(),
        }
    }

    pub fn continue_with(&mut self, input: &'a str) -> &mut Self {
        self.parser = AnsiParser::new(input);
        self
    }
}

impl<'a> Iterator for AnsiParserWithStyle<'a> {
    type Item = (&'a str, Style);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(event) = self.parser.next() {
            match event {
                AnsiEvent::Text(s) => {
                    return Some((s, self.style));
                }
                AnsiEvent::Tag(tag) => {
                    tag.apply_to_style(&mut self.style);
                }
            }
        }

        None
    }
}

pub struct AnsiViewer {
    ansi: String,
    scroll: u16,
    hash: u64,
    size: Size,
    view: Size,
    lines: u16,
    scroll_area: Option<Rect>,
    mode: AnsiViewMode,
    span: TextSpan,
    options: AnsiViewerOptions,
    colors: AnsiViewerColors,
}

impl AnsiViewer {
    pub const fn new() -> Self {
        Self {
            ansi: String::new(),
            scroll: 0,
            hash: 0,
            size: Size::ZERO,
            view: Size::ZERO,
            lines: 0,
            scroll_area: None,
            mode: AnsiViewMode::DEFAULT,
            span: TextSpan::new(),
            options: AnsiViewerOptions::new(),
            colors: AnsiViewerColors::new(),
        }
    }

    pub const fn with_padding(mut self, padding: Padding) -> Self {
        self.set_padding(padding);
        self
    }

    pub const fn with_text_alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.set_text_alignment(alignment);
        self
    }

    pub const fn with_colors(mut self, colors: AnsiViewerColors) -> Self {
        self.set_colors(colors);
        self
    }

    pub const fn set_padding(&mut self, padding: Padding) -> &mut Self {
        self.options.padding = padding;
        self
    }

    pub const fn set_text_alignment(&mut self, alignment: HorizontalAlignment) -> &mut Self {
        self.options.text_alignment = alignment;
        self
    }

    pub const fn set_colors(&mut self, colors: AnsiViewerColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub const fn set_view_mode(&mut self, mode: AnsiViewMode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub const fn current_scroll(&self) -> u16 {
        self.scroll
    }

    pub fn input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Down => self.scroll(ScrollMove::Down),
            KeyCode::Up => self.scroll(ScrollMove::Up),
            KeyCode::PageDown => self.scroll(ScrollMove::PageDown),
            KeyCode::PageUp => self.scroll(ScrollMove::PageUp),
            KeyCode::Home => self.scroll(ScrollMove::Start),
            KeyCode::End => self.scroll(ScrollMove::End),
            _ => false,
        }
    }

    pub fn scroll(&mut self, sm: ScrollMove) -> bool {
        let old_scroll = self.scroll;
        let height = self.view.height;

        self.scroll = match sm {
            ScrollMove::Up => self.scroll.saturating_sub(1),
            ScrollMove::Down => (self.scroll + 1).min(self.lines.saturating_sub(height)),
            ScrollMove::PageUp => self.scroll.saturating_sub(height),
            ScrollMove::PageDown => (self.scroll + height).min(self.lines.saturating_sub(height)),
            ScrollMove::Start => 0,
            ScrollMove::End => self.lines.saturating_sub(height),
        };

        self.scroll != old_scroll
    }

    pub fn render(&mut self, mut area: Rect, buf: &mut Buffer, ansi: &str) {
        let mut inner = area.inner_padding(self.options.padding);

        if inner.is_empty() {
            return;
        }

        let hash = utils::hash_fast(ansi);
        if self.hash != hash || self.size != area.as_size() {
            self.size = area.as_size();
            self.hash = utils::hash_fast(ansi);
            self.view = inner.as_size();
            self.scroll_area = None;
            self.relayout(inner.width, ansi);
        }

        let skip = match self.mode {
            AnsiViewMode::Text { center_vertical } => {
                if center_vertical {
                    inner.height = self.lines.min(inner.height);
                    inner.y = area.y + (area.height.saturating_sub(inner.height)) / 2;
                }

                0
            }
            AnsiViewMode::Page {
                scrollbar,
                scrollbar_margin,
            } => {
                let is_scrollable = scrollbar
                    && Scrollbar::is_scrollable(ScrollableData::new(
                        self.lines as usize,
                        inner.as_size(),
                    ));
                if is_scrollable {
                    let scroll_area =
                        Scrollbar::make_scroll_area_with_margin(&mut area, scrollbar_margin);
                    inner.width = inner
                        .width
                        .saturating_sub(scroll_area.width + scrollbar_margin);
                    self.view = inner.as_size();
                    self.scroll_area = Some(scroll_area);
                    self.relayout(inner.width, ansi);
                }

                self.update_scroll();
                self.render_scrollbar(buf);

                self.scroll
            }
        };

        let mut ansi_parser = AnsiParser::new("").with_style();
        let lines = self
            .ansi
            .lines()
            .skip(skip as usize)
            .take(inner.height as usize);

        match self.options.text_alignment {
            HorizontalAlignment::Left => {
                let Rect { mut x, mut y, .. } = inner;
                lines.for_each(|line| {
                    ansi_parser.continue_with(line);

                    for (s, style) in ansi_parser.continue_with(line) {
                        (x, _) = buf.set_stringn(x, y, s, usize::MAX, style);
                    }

                    x = inner.x;
                    y += 1;
                });
            }
            HorizontalAlignment::Center | HorizontalAlignment::Right => {
                self.span.set_alignment(self.options.text_alignment);
                let mut line_area = Rect { height: 1, ..inner };
                lines.for_each(|line| {
                    ansi_parser.continue_with(line);

                    for (s, style) in ansi_parser.continue_with(line) {
                        self.span.push_str(s, style);
                    }

                    self.span.render(line_area, buf);
                    self.span.clear();
                    line_area.y += 1;
                });
            }
        }
    }

    fn relayout(&mut self, max_width: u16, ansi: &str) {
        self.ansi.clear();

        self.ansi.push_str(ansi);
        textwrap::fill_inplace(&mut self.ansi, max_width as usize);
        self.lines = self.ansi.lines().count() as u16;
    }

    fn update_scroll(&mut self) {
        let height = self.view.height;
        self.scroll = self.scroll.min(self.lines.saturating_sub(height));
    }

    fn render_scrollbar(&self, buf: &mut Buffer) {
        let Some(scroll_area) = self.scroll_area else {
            return;
        };

        Scrollbar::new(ScrollbarData {
            viewport_height: self.size.height,
            current_scroll: self.scroll as usize,
            total_items: self.lines as usize,
        })
        .with_colors(self.colors.scrollbar)
        .render(scroll_area, buf);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnsiViewMode {
    // Span TODO
    Text {
        center_vertical: bool,
    },
    Page {
        scrollbar: bool,
        scrollbar_margin: u16,
    },
}

impl AnsiViewMode {
    pub const DEFAULT: Self = Self::Text {
        center_vertical: false,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct AnsiViewerOptions {
    pub padding: Padding,
    pub text_alignment: HorizontalAlignment,
}

impl AnsiViewerOptions {
    pub const fn new() -> Self {
        Self {
            padding: Padding::ZERO,
            text_alignment: HorizontalAlignment::Left,
        }
    }
}

impl Default for AnsiViewerOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnsiViewerColors {
    pub scrollbar: ScrollbarColors,
}

impl AnsiViewerColors {
    pub const fn new() -> Self {
        Self {
            scrollbar: ScrollbarColors::DEFAULT,
        }
    }
}

impl Default for AnsiViewerColors {
    fn default() -> Self {
        Self::new()
    }
}
