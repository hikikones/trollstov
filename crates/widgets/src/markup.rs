use std::{
    ops::{Range, RangeInclusive},
    path::{Path, PathBuf},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{Alignment, Rect, Size},
    style::Color,
    widgets::Padding,
};
use shared::terminal::{TerminalCellSize, TerminalPalette};
use syntect::{
    easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings,
};
use unicode_segmentation::UnicodeSegmentation;
use utils::Formatter;

use crate::{
    RectExt, ScrollableData, Scrollbar, ScrollbarColors, ScrollbarData,
    ansi::{AnsiParser, AnsiTag, AnsiWriter},
    kitty_graphics::{Dimensions, KittyGraphics, ResizeMode},
    text_span::TextSpan,
};

pub struct Markup {
    plain: MarkupPlainData,
    rich: MarkupRichData,
    scroll: MarkupScroll,
    kitty: MarkupKitty,
    math: MarkupMath,
    cache: MarkupCache,
    options: MarkupOptions,
    colors: MarkupColors,
    assets: PathBuf,
}

impl Markup {
    pub fn new(assets: PathBuf, cell_size: TerminalCellSize, palette: TerminalPalette) -> Self {
        Self {
            plain: MarkupPlainData::new(),
            rich: MarkupRichData::new(),
            scroll: MarkupScroll::new(),
            kitty: MarkupKitty::new(),
            math: MarkupMath::new(cell_size, palette),
            cache: MarkupCache::new(),
            options: MarkupOptions::new(),
            colors: MarkupColors::new(),
            assets,
        }
    }

    pub const fn with_options(mut self, options: MarkupOptions) -> Self {
        self.options = options;
        self
    }

    pub const fn with_colors(mut self, colors: MarkupColors) -> Self {
        self.colors = colors;
        self
    }

    pub const fn scroll_index(&self) -> u16 {
        self.scroll.current
    }

    pub const fn set_desired_scroll(&mut self, sm: ScrollMove) -> &mut Self {
        self.scroll.desired = Some(sm);
        self
    }

    pub const fn set_colors(&mut self, colors: MarkupColors) -> &mut Self {
        self.colors = colors;
        self
    }

    pub const fn set_max_items(&mut self, max: Option<usize>) -> &mut Self {
        self.scroll.max_items = max;
        self
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
        let old_scroll = self.scroll.current;
        self.scroll.set(sm, self.cache.area.height);
        self.scroll.current != old_scroll
    }

    pub fn render(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        markup: &str,
        kitty: &mut KittyGraphics,
    ) {
        if area.inner_padding(self.options.padding).is_empty() {
            return;
        }

        let hash = utils::hash_fast(markup);
        if self.cache.size != area.as_size() || self.cache.hash != hash {
            self.cache.size = area.as_size();
            self.cache.area = area.inner_padding(self.options.padding);
            self.cache.hash = hash;
            self.cache.scroll_area = None;

            self.parse_and_load(markup, kitty);
            self.process_markup(kitty);

            if self.is_scrollable() {
                let scroll_area = Scrollbar::make_scroll_area_with_margin(
                    &mut area,
                    self.options.scrollbar_margin,
                );
                self.cache.area.width = self
                    .cache
                    .area
                    .width
                    .saturating_sub(scroll_area.width + self.options.scrollbar_margin);
                self.cache.scroll_area = Some(scroll_area);
                self.process_markup(kitty);
            }
        }

        // Scroll
        self.update_scroll(kitty);
        self.render_scrollbar(buf);

        // Setup
        let mut area = self.cache.area;

        let top_y = area.y;
        let viewport_top = self.scroll.current;
        let viewport_bot = self.scroll.current + area.height;
        let mut current_line = 0;

        // Render
        for item in self.rich.items.iter().cloned().take(self.max_items()) {
            if area.height == 0 {
                break;
            }

            const fn is_in_viewport(curr_line: u16, top: u16, bot: u16) -> bool {
                curr_line >= top && curr_line < bot
            }

            match item {
                MarkupRich::Text { range, alignment } => {
                    let mut ansi_parser = AnsiParser::new("").with_style();
                    self.rich.span.set_alignment(alignment);

                    for line in self.rich.formatter.slice(range).lines() {
                        let is_in_viewport =
                            is_in_viewport(current_line, viewport_top, viewport_bot);

                        for (s, style) in ansi_parser.continue_with(line) {
                            if is_in_viewport {
                                self.rich.span.push_str(s, style);
                            }
                        }

                        if is_in_viewport {
                            self.rich.span.render(area, buf);
                            self.rich.span.clear();
                            area.shrink_down(1);
                        }

                        current_line += 1;
                    }
                }
                MarkupRich::Image { id, dims } => {
                    let max_width = kitty.width(area.width);
                    let resized_dims = KittyGraphics::resize(dims, dims.with_width(max_width));
                    let resized_rows = kitty.rows(resized_dims.height);

                    if is_in_viewport(
                        current_line,
                        viewport_top.saturating_sub(resized_rows - 1),
                        viewport_bot,
                    ) {
                        let is_at_top = area.y == top_y;
                        let rows_outside_top = if is_at_top {
                            current_line.abs_diff(self.scroll.current)
                        } else {
                            0
                        };
                        let image_rows = (resized_rows - rows_outside_top).min(area.height);
                        let image_area = Rect {
                            height: image_rows,
                            ..area
                        };
                        kitty.render(
                            image_area,
                            buf,
                            id,
                            dims,
                            ResizeMode::FitWidthCropHeight { rows_outside_top },
                            crate::utils::Alignment::CenterHorizontal,
                        );
                        self.kitty.has_rendered = true;
                        area.shrink_down(image_rows);
                    }

                    current_line += resized_rows;
                }
                MarkupRich::Break => {
                    if is_in_viewport(current_line, viewport_top, viewport_bot) {
                        crate::print_char_repeat(
                            area,
                            buf,
                            self.options.break_char,
                            area.width,
                            self.colors.break_char,
                        );
                        area.shrink_down(1);
                    }

                    current_line += 1;
                }
                MarkupRich::EmptyLine => {
                    if is_in_viewport(current_line, viewport_top, viewport_bot) {
                        area.shrink_down(1);
                    }

                    current_line += 1;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.plain.clear();
        self.rich.clear();
        self.scroll.clear();
        self.cache.clear();
    }

    pub fn delete_images(&mut self, kitty: &KittyGraphics) -> std::io::Result<()> {
        if self.kitty.has_rendered {
            kitty.delete_range(self.kitty.range())?;
            self.kitty.has_rendered = false;
        }
        Ok(())
    }

    fn parse_and_load(&mut self, markup: &str, kitty: &mut KittyGraphics) {
        self.plain.clear();
        self.kitty.id_counter = 0;

        // Convert tabs to spaces before we parse and load markup
        let markup = if markup.contains('\t') {
            self.plain.buffer.extend(
                markup
                    .graphemes(true)
                    .map(|g| if g == "\t" { "    " } else { g }),
            );
            self.plain.buffer.as_str()
        } else {
            markup
        };

        // Parse and load markup
        for (block, _) in BlockParser::new(markup) {
            match block {
                BlockElement::Paragraph { text, alignment } => {
                    self.plain.items.push(MarkupPlain::Paragraph {
                        text: self.plain.formatter.push_str(text),
                        alignment,
                    });
                }
                BlockElement::Heading { text, alignment } => {
                    // TODO: No empty line after heading
                    self.plain.items.push(MarkupPlain::Heading {
                        text: self.plain.formatter.push_str(text),
                        alignment,
                    });
                }
                BlockElement::List { items } => {
                    for item in items {
                        self.plain.items.push(MarkupPlain::ListItem {
                            text: self.plain.formatter.push_str(item),
                        });
                    }
                }
                BlockElement::Code { language, text } => {
                    self.plain.items.push(MarkupPlain::Code {
                        text: self.plain.formatter.push_str(text),
                        _language: self.plain.formatter.push_str(language),
                    });
                }
                BlockElement::Image { description, path } => {
                    let image_path = Path::new(path)
                        .file_name()
                        .map(|name| self.assets.join(name));

                    fn load_and_encode_image(
                        path: Option<PathBuf>,
                        id: u32,
                        kitty: &mut KittyGraphics,
                    ) -> Result<Dimensions, String> {
                        let Some(path) = path else {
                            return Err(String::from("No image filename found"));
                        };
                        kitty.load(&path).map_err(|err| {
                            format!(
                                "Failed to load image\n\"{}\"\ndue to\n\"{}\"",
                                path.display(),
                                err
                            )
                        })?;
                        let dims = kitty.encode(id).map_err(|err| {
                            format!(
                                "Failed to encode image\n\"{}\"\ndue to\n\"{}\"",
                                path.display(),
                                err
                            )
                        })?;
                        Ok(dims)
                    }

                    let id = self.kitty.current_id();
                    match load_and_encode_image(image_path, id, kitty) {
                        Ok(dims) => {
                            self.plain.items.push(MarkupPlain::Image { id, dims });
                            self.kitty.increment_id();
                        }
                        Err(err) => {
                            self.plain.items.push(MarkupPlain::ImageError {
                                text: self.plain.formatter.extend(["ERROR\n", err.as_str()]),
                            });
                        }
                    }

                    if !description.is_empty() {
                        self.plain.items.push(MarkupPlain::ImageDescription {
                            text: self.plain.formatter.push_str(description),
                        });
                    }
                }
                BlockElement::Math { text } => {
                    fn parse_load_and_encode(
                        text: &str,
                        math: &MarkupMath,
                        id: u32,
                        kitty: &mut KittyGraphics,
                    ) -> Result<Dimensions, String> {
                        let ast = math.parse(text).map_err(|err| {
                            format!("Failed to parse math\n\"{text}\"\ndue to\n\"{}\"", err)
                        })?;
                        let png = math.to_png(ast).map_err(|err| {
                            format!("Failed to make math image due to\n\"{}\"", err)
                        })?;
                        kitty.load_png_from_bytes(png).map_err(|err| {
                            format!("Failed to load math image due to\n\"{}\"", err)
                        })?;
                        let dims = kitty.encode(id).map_err(|err| {
                            format!("Failed to encode math image due to\n\"{}\"", err)
                        })?;
                        Ok(dims)
                    }

                    let id = self.kitty.current_id();
                    match parse_load_and_encode(text, &self.math, id, kitty) {
                        Ok(dims) => {
                            self.plain.items.push(MarkupPlain::Image { id, dims });
                            self.kitty.increment_id();
                        }
                        Err(err) => {
                            self.plain.items.push(MarkupPlain::ImageError {
                                text: self.plain.formatter.extend(["ERROR\n", err.as_str()]),
                            });
                        }
                    }
                }
                BlockElement::Break => {
                    self.plain.items.push(MarkupPlain::Break);
                }
                BlockElement::Comment { .. } => continue,
            }

            // Add empty line between each block element
            self.plain.items.push(MarkupPlain::EmptyLine);
        }

        // Remove last empty line
        self.plain.items.pop();
    }

    fn process_markup(&mut self, kitty: &KittyGraphics) {
        self.rich.clear();

        let width = self.cache.area.width;

        // Process parsed markup
        self.rich
            .items
            .extend(self.plain.items.iter().cloned().map(|item| match item {
                MarkupPlain::Paragraph { text, alignment } => MarkupRich::Text {
                    range: markup_to_rich_ansi(
                        self.plain.formatter.slice(text),
                        width,
                        None,
                        &mut self.rich.writer,
                        &mut self.rich.formatter,
                    ),
                    alignment,
                },
                MarkupPlain::Heading { text, alignment } => {
                    self.rich.writer.push_tag(AnsiTag::Bold);
                    self.rich.writer.push_tag(AnsiTag::FgYellow); // TODO: Use secondary color
                    self.rich.writer.push_str(self.plain.formatter.slice(text));

                    self.rich.writer.textwrap(width);

                    let range = self.rich.formatter.push_str(self.rich.writer.as_str());
                    self.rich.writer.clear();

                    MarkupRich::Text { range, alignment }
                }
                MarkupPlain::ListItem { text } => MarkupRich::Text {
                    range: markup_to_rich_ansi(
                        self.plain.formatter.slice(text),
                        width.saturating_sub(4),
                        Some(("  • ", "    ")),
                        &mut self.rich.writer,
                        &mut self.rich.formatter,
                    ),
                    alignment: Alignment::Left,
                },
                MarkupPlain::Code { text, _language } => {
                    let lang = self.plain.formatter.slice(_language);
                    let code = self.plain.formatter.slice(text);

                    self.rich.highlighter.highlight(
                        lang,
                        code,
                        self.colors.syntax_theme,
                        |span, color| match color {
                            Some((r, g, b)) => {
                                self.rich.writer.push_tag(AnsiTag::FgTrueColor(r, g, b));
                                self.rich.writer.push_str(span);
                                self.rich.writer.push_tag(AnsiTag::Reset);
                            }
                            None => {
                                self.rich.writer.push_str(span);
                            }
                        },
                    );

                    let range = self.rich.formatter.push_str(self.rich.writer.as_str());
                    self.rich.writer.clear();

                    MarkupRich::Text {
                        range,
                        alignment: Alignment::Left,
                    }
                }
                MarkupPlain::Image { id, dims } => MarkupRich::Image { id, dims },
                MarkupPlain::ImageError { text } => {
                    self.rich.writer.push_tag(AnsiTag::FgRed);
                    self.rich.writer.push_str(self.plain.formatter.slice(text));

                    self.rich.writer.textwrap(width);

                    let range = self.rich.formatter.push_str(self.rich.writer.as_str());
                    self.rich.writer.clear();

                    MarkupRich::Text {
                        range,
                        alignment: Alignment::Center,
                    }
                }
                MarkupPlain::ImageDescription { text } => MarkupRich::Text {
                    range: markup_to_rich_ansi(
                        self.plain.formatter.slice(text),
                        width,
                        None,
                        &mut self.rich.writer,
                        &mut self.rich.formatter,
                    ),
                    alignment: Alignment::Center,
                },
                MarkupPlain::Break => MarkupRich::Break,
                MarkupPlain::EmptyLine => MarkupRich::EmptyLine,
            }));

        // Compute total lines
        self.scroll.total_lines = self.compute_total_lines(kitty);

        // Helper function
        fn markup_to_rich_ansi(
            markup: &str,
            width: u16,
            indent: Option<(&str, &str)>,
            writer: &mut AnsiWriter,
            storage: &mut Formatter,
        ) -> Range<usize> {
            // Convert markup to ansi
            for event in InlineParser::new(markup) {
                match event {
                    InlineEvent::Text(s) => writer.push_str(s),
                    InlineEvent::Tag(tag) => writer.push_tag(tag.into_ansi()),
                }
            }

            // Break text into lines using textwrap which ignores ansi codes
            writer.textwrap(width);

            // Store result for later usage
            let range = match indent {
                Some((first_indent, other_indent)) => {
                    let start = storage.len();
                    for (i, line) in writer.as_str().lines().enumerate() {
                        let indent = if i == 0 { first_indent } else { other_indent };
                        storage.extend([indent, line, "\n"]);
                    }
                    start..storage.len()
                }
                None => storage.push_str(writer.as_str()),
            };
            writer.clear();
            range
        }
    }

    const fn is_scrollable(&self) -> bool {
        self.options.scrollbar
            && Scrollbar::is_scrollable(ScrollableData::new(
                self.scroll.total_lines as usize,
                self.cache.area.as_size(),
            ))
    }

    fn update_scroll(&mut self, kitty: &KittyGraphics) {
        self.scroll.max_lines = self.compute_max_lines(kitty);

        let height = self.cache.area.height;
        match self.scroll.desired.take() {
            Some(sm) => {
                self.scroll.set(sm, height);
            }
            None => {
                self.scroll.current = self
                    .scroll
                    .current
                    .min(self.scroll.max_lines.saturating_sub(height));
            }
        }
    }

    fn render_scrollbar(&self, buf: &mut Buffer) {
        let Some(scroll_area) = self.cache.scroll_area else {
            return;
        };

        Scrollbar::new(ScrollbarData {
            viewport_height: self.cache.area.height,
            current_scroll: self.scroll.current as usize,
            total_items: self.scroll.total_lines as usize,
        })
        .with_colors(self.colors.scrollbar)
        .render(scroll_area, buf);
    }

    fn compute_max_lines(&self, kitty: &KittyGraphics) -> u16 {
        self.compute_lines(self.max_items(), self.cache.area.width, kitty)
    }

    fn compute_total_lines(&self, kitty: &KittyGraphics) -> u16 {
        self.compute_lines(self.rich.items.len(), self.cache.area.width, kitty)
    }

    fn compute_lines(&self, max_items: usize, max_width: u16, kitty: &KittyGraphics) -> u16 {
        self.rich
            .items
            .iter()
            .cloned()
            .take(max_items)
            .map(|item| match item {
                MarkupRich::Text { range, .. } => {
                    self.rich.formatter.slice(range).lines().count() as u16
                }
                MarkupRich::Image { dims, .. } => {
                    let max_width = kitty.width(max_width);
                    let resized_dims = KittyGraphics::resize(dims, dims.with_width(max_width));
                    kitty.rows(resized_dims.height)
                }
                MarkupRich::Break | MarkupRich::EmptyLine => 1,
            })
            .sum()
    }

    const fn max_items(&self) -> usize {
        match self.scroll.max_items {
            Some(max) => max,
            None => self.rich.items.len(),
        }
    }

    /// Returns the index before every break point in the `markup`,
    /// including the last index. Useful for [`set_max_items`](Self::set_max_items).
    pub fn parse_break_points(markup: &str) -> impl Iterator<Item = usize> {
        struct Breaks<'a> {
            blocks: BlockParser<'a>,
            len: usize,
        }

        impl<'a> Breaks<'a> {
            fn new(markup: &'a str) -> Self {
                Self {
                    blocks: BlockParser::new(markup),
                    len: 0,
                }
            }
        }

        impl<'a> Iterator for Breaks<'a> {
            type Item = usize;

            fn next(&mut self) -> Option<Self::Item> {
                let len = self.len;

                while let Some((block, _)) = self.blocks.next() {
                    match block {
                        BlockElement::Paragraph { .. }
                        | BlockElement::Code { .. }
                        | BlockElement::Math { .. } => {
                            self.len += 1;
                        }
                        BlockElement::Heading { .. } => {
                            self.len += 1; // TODO: No empty line after heading
                        }
                        BlockElement::List { items } => {
                            self.len += items.count();
                        }
                        BlockElement::Image { description, .. } => {
                            self.len += 1;
                            if !description.is_empty() {
                                self.len += 1;
                            }
                        }
                        BlockElement::Comment { .. } => continue,
                        BlockElement::Break => {
                            let i = self.len;
                            self.len += 2;
                            return Some(i);
                        }
                    }

                    // EmptyLine
                    self.len += 1;
                }

                if len == self.len {
                    None
                } else {
                    // No last EmptyLine
                    Some(self.len.saturating_sub(1))
                }
            }
        }

        Breaks::new(markup)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollMove {
    Up,
    Down,
    PageUp,
    PageDown,
    Start,
    End,
}

#[derive(Debug, Clone)]
enum MarkupPlain {
    Paragraph {
        text: Range<usize>,
        alignment: Alignment,
    },
    Heading {
        text: Range<usize>,
        alignment: Alignment,
    },
    ListItem {
        text: Range<usize>,
    },
    Code {
        text: Range<usize>,
        _language: Range<usize>,
    },
    Image {
        id: u32,
        dims: Dimensions,
    },
    ImageError {
        text: Range<usize>,
    },
    ImageDescription {
        text: Range<usize>,
    },
    Break,
    EmptyLine,
}

struct MarkupPlainData {
    items: Vec<MarkupPlain>,
    formatter: Formatter,
    buffer: String,
}

impl MarkupPlainData {
    const fn new() -> Self {
        Self {
            items: Vec::new(),
            formatter: Formatter::new(),
            buffer: String::new(),
        }
    }

    fn clear(&mut self) {
        self.items.clear();
        self.formatter.clear();
        self.buffer.clear();
    }
}

#[derive(Debug, Clone)]
enum MarkupRich {
    Text {
        range: Range<usize>,
        alignment: Alignment,
    },
    Image {
        id: u32,
        dims: Dimensions,
    },
    Break,
    EmptyLine,
}

struct MarkupRichData {
    items: Vec<MarkupRich>,
    writer: AnsiWriter,
    formatter: Formatter,
    span: TextSpan,
    highlighter: CodeHighlighter,
}

impl MarkupRichData {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            writer: AnsiWriter::new(),
            formatter: Formatter::new(),
            span: TextSpan::new(),
            highlighter: CodeHighlighter::new(),
        }
    }

    fn clear(&mut self) {
        self.items.clear();
        self.writer.clear();
        self.formatter.clear();
        self.span.clear();
    }
}

struct MarkupScroll {
    current: u16,
    desired: Option<ScrollMove>,
    max_items: Option<usize>,
    max_lines: u16,
    total_lines: u16,
}

impl MarkupScroll {
    const fn new() -> Self {
        Self {
            current: 0,
            desired: None,
            max_items: None,
            max_lines: 0,
            total_lines: 0,
        }
    }

    fn set(&mut self, sm: ScrollMove, viewport_height: u16) {
        self.current = match sm {
            ScrollMove::Up => self.current.saturating_sub(1),
            ScrollMove::Down => {
                (self.current + 1).min(self.max_lines.saturating_sub(viewport_height))
            }
            ScrollMove::PageUp => self.current.saturating_sub(viewport_height),
            ScrollMove::PageDown => {
                (self.current + viewport_height).min(self.max_lines.saturating_sub(viewport_height))
            }
            ScrollMove::Start => 0,
            ScrollMove::End => self.max_lines.saturating_sub(viewport_height),
        };
    }

    fn clear(&mut self) {
        self.current = 0;
        self.desired = None;
        self.max_items = None;
        self.max_lines = 0;
        self.total_lines = 0;
    }
}

struct MarkupKitty {
    id_start: u32,
    id_counter: u32,
    has_rendered: bool,
}

impl MarkupKitty {
    const fn new() -> Self {
        Self {
            id_start: 90,
            id_counter: 0,
            has_rendered: false,
        }
    }

    const fn current_id(&self) -> u32 {
        self.id_start + self.id_counter
    }

    const fn increment_id(&mut self) {
        self.id_counter += 1;
    }

    const fn range(&self) -> RangeInclusive<u32> {
        let end = self.id_start + self.id_counter.saturating_sub(1);
        self.id_start..=end
    }
}

struct MarkupMath {
    layout_options: ratex_layout::LayoutOptions,
    render_options: ratex_render::RenderOptions,
}

pub enum MarkupMathColor {
    White,
    Black,
    Rgb(u8, u8, u8),
}

impl MarkupMath {
    const fn new(size: TerminalCellSize, palette: TerminalPalette) -> Self {
        Self {
            layout_options: ratex_layout::LayoutOptions {
                style: ratex_types::MathStyle::Display,
                color: ratex_types::Color {
                    r: palette.foreground.r as f32 / 255.0,
                    g: palette.foreground.g as f32 / 255.0,
                    b: palette.foreground.b as f32 / 255.0,
                    a: 1.0,
                },
                align_relation_spacing: None,
                leftright_delim_height: None,
                inter_glyph_kern_em: 0.0,
            },
            render_options: ratex_render::RenderOptions {
                font_size: size.width as f32 * 1.2,
                padding: 0.0,
                background_color: ratex_types::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                },
                font_dir: String::new(),
                device_pixel_ratio: size.height as f32 / size.width as f32,
            },
        }
    }

    fn parse(&self, math: &str) -> ratex_parser::ParseResult<Vec<ratex_parser::ParseNode>> {
        ratex_parser::parse(math)
    }

    fn to_png(&self, ast: Vec<ratex_parser::ParseNode>) -> Result<Vec<u8>, String> {
        let layout = ratex_layout::layout(&ast, &self.layout_options);
        let display_list = ratex_layout::to_display_list(&layout);
        ratex_render::render_to_png(&display_list, &self.render_options)
    }
}

struct MarkupCache {
    size: Size,
    area: Rect,
    hash: u64,
    scroll_area: Option<Rect>,
}

impl MarkupCache {
    const fn new() -> Self {
        Self {
            size: Size::ZERO,
            area: Rect::ZERO,
            hash: 0,
            scroll_area: None,
        }
    }

    const fn clear(&mut self) {
        self.size = Size::ZERO;
        self.area = Rect::ZERO;
        self.hash = 0;
        self.scroll_area = None;
    }
}

pub struct MarkupOptions {
    pub padding: Padding,
    pub scrollbar: bool,
    pub scrollbar_margin: u16,
    pub break_char: char,
}

impl MarkupOptions {
    pub const fn new() -> Self {
        Self {
            padding: Padding::ZERO,
            scrollbar: false,
            scrollbar_margin: 1,
            break_char: '─',
        }
    }
}

impl Default for MarkupOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MarkupColors {
    pub syntax_theme: SyntaxHighlightTheme,
    pub scrollbar: ScrollbarColors,
    pub break_char: Color,
}

impl MarkupColors {
    pub const fn new() -> Self {
        Self {
            syntax_theme: SyntaxHighlightTheme::Base16EightiesDark,
            scrollbar: ScrollbarColors::DEFAULT,
            break_char: Color::Indexed(240),
        }
    }
}

impl Default for MarkupColors {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum BlockElement<'a> {
    Paragraph { text: &'a str, alignment: Alignment },
    Heading { text: &'a str, alignment: Alignment },
    List { items: ListItems<'a> },
    Code { language: &'a str, text: &'a str },
    // TODO: Table
    Image { description: &'a str, path: &'a str },
    Math { text: &'a str },
    Comment { _text: &'a str },
    Break,
}

struct BlockParser<'a> {
    input: &'a str,
    graphemes: utils::PeekableGraphemesPrevious<'a>,
}

impl<'a> BlockParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            graphemes: utils::PeekableGraphemesPrevious::new(input),
        }
    }

    fn parse_paragraph(
        &mut self,
        start: usize,
        alignment: Alignment,
    ) -> (BlockElement<'a>, Range<usize>) {
        let start_offset = match alignment {
            Alignment::Left => 0,
            Alignment::Center | Alignment::Right => 1,
        };
        let paragraph_start = start + start_offset;
        let (paragraph_end, end) = match self.graphemes.find_consecutive_by(|g| g.contains('\n'), 2)
        {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            BlockElement::Paragraph {
                text: self.input[paragraph_start..paragraph_end].trim(),
                alignment,
            },
            start..end,
        );
    }

    fn parse_heading(&mut self, start: usize) -> (BlockElement<'a>, Range<usize>) {
        let count = self.graphemes.count_consecutive("=", usize::MAX);
        let (alignment, offset) = match self.graphemes.next() {
            Some((_, g)) => match g {
                "|" => (Alignment::Center, 1),
                ">" => (Alignment::Right, 1),
                _ => (Alignment::Left, 0),
            },
            None => return self.parse_paragraph(start, Alignment::Left),
        };

        let heading_start = start + 1 + count + offset;
        let (heading_end, end) = match self.graphemes.find_newline() {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            BlockElement::Heading {
                text: self.input[heading_start..heading_end].trim(),
                alignment,
            },
            start..end,
        );
    }

    fn parse_list(&mut self, start: usize) -> (BlockElement<'a>, Range<usize>) {
        let list_start = start + 1;
        let (list_end, end) = match self.graphemes.find_consecutive_by(|g| g.contains('\n'), 2) {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            BlockElement::List {
                items: ListItems::new(self.input[list_start..list_end].trim()),
            },
            start..end,
        );
    }

    fn parse_comment(&mut self, start: usize) -> (BlockElement<'a>, Range<usize>) {
        let comment_start = start + 1;
        let (comment_end, end) = match self.graphemes.find_by(|g| g.contains('\n')) {
            Some((i, g)) => (i, i + g.len()),
            None => (self.input.len(), self.input.len()),
        };

        return (
            BlockElement::Comment {
                _text: self.input[comment_start..comment_end].trim(),
            },
            start..end,
        );
    }

    fn parse_code_block(&mut self, start: usize, ticks: usize) -> (BlockElement<'a>, Range<usize>) {
        let lang_start = start + ticks;
        let Some((i, g)) = self.graphemes.find_by(|g| g.contains('\n')) else {
            return (
                BlockElement::Code {
                    language: self.input[lang_start..].trim(),
                    text: "",
                },
                start..self.input.len(),
            );
        };

        let language = self.input[lang_start..i].trim();
        let code_start = i + g.len();
        loop {
            let Some((code_end, g)) = self.graphemes.find_by(|g| g.contains('\n')) else {
                return (
                    BlockElement::Code {
                        language,
                        text: &self.input[code_start..],
                    },
                    start..self.input.len(),
                );
            };

            let end_ticks = self.graphemes.count_consecutive("`", usize::MAX);
            if end_ticks == ticks {
                let end = code_end + g.len() + end_ticks;
                let mut graphemes = self.input[end..].grapheme_indices(true);

                let Some((i, g)) = graphemes.next() else {
                    return (
                        BlockElement::Code {
                            language,
                            text: &self.input[code_start..code_end],
                        },
                        start..end,
                    );
                };

                if g.contains('\n') {
                    let Some((i, g)) = graphemes.next() else {
                        return (
                            BlockElement::Code {
                                language,
                                text: &self.input[code_start..code_end],
                            },
                            start..i + g.len(),
                        );
                    };

                    if g.contains('\n') {
                        self.graphemes.next();
                        self.graphemes.next();

                        return (
                            BlockElement::Code {
                                language,
                                text: &self.input[code_start..code_end],
                            },
                            start..i + g.len(),
                        );
                    }
                }
            }
        }
    }

    fn parse_image(&mut self, start: usize) -> (BlockElement<'a>, Range<usize>) {
        let Some((descr_start, "[")) = self.graphemes.next() else {
            return self.parse_paragraph(start, Alignment::Left);
        };
        let Some((descr_end, _)) = self.graphemes.find("]") else {
            return self.parse_paragraph(start, Alignment::Left);
        };

        let Some((path_start, "(")) = self.graphemes.next() else {
            return self.parse_paragraph(start, Alignment::Left);
        };
        let Some((path_end, _)) = self.graphemes.find(")") else {
            return self.parse_paragraph(start, Alignment::Left);
        };

        let description = self.input[descr_start + 1..descr_end].trim();
        let path = self.input[path_start + 1..path_end].trim();

        let Some((end, g)) = self.graphemes.next() else {
            return (
                BlockElement::Image { description, path },
                start..self.input.len(),
            );
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, Alignment::Left);
        }

        let Some((_, g)) = self.graphemes.next() else {
            return (BlockElement::Image { description, path }, start..end);
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, Alignment::Left);
        }

        (BlockElement::Image { description, path }, start..end)
    }

    fn parse_math(&mut self, start: usize) -> (BlockElement<'a>, Range<usize>) {
        let Some((end_math, _)) = self.graphemes.find_consecutive("$", 2) else {
            return self.parse_paragraph(start, Alignment::Left);
        };

        let start_math = start + 2;
        let end_math = end_math - 1;
        let text = self.input[start_math..end_math].trim();

        if text.is_empty() {
            return self.parse_paragraph(start, Alignment::Left);
        }

        let Some((end, g)) = self.graphemes.next() else {
            return (BlockElement::Math { text }, start..self.input.len());
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, Alignment::Left);
        }

        let Some((_, g)) = self.graphemes.next() else {
            return (BlockElement::Math { text }, start..end);
        };

        if !g.contains("\n") {
            return self.parse_paragraph(start, Alignment::Left);
        }

        (BlockElement::Math { text }, start..end)
    }
}

impl<'a> Iterator for BlockParser<'a> {
    type Item = (BlockElement<'a>, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((i, g)) = self.graphemes.next() {
            if g.chars().any(|c| c.is_whitespace()) {
                continue;
            }

            if let Some("\n") | Some("\r\n") | None = self.graphemes.previous() {
                let (block, range) = match g {
                    "|" => self.parse_paragraph(i, Alignment::Center),
                    ">" => self.parse_paragraph(i, Alignment::Right),
                    "=" => self.parse_heading(i),
                    "#" => self.parse_comment(i),
                    "!" => self.parse_image(i),
                    "`" => {
                        let ticks = 1 + self.graphemes.count_consecutive("`", usize::MAX);
                        if ticks >= 3 {
                            self.parse_code_block(i, ticks)
                        } else {
                            self.parse_paragraph(i, Alignment::Left)
                        }
                    }
                    "-" => {
                        let dashes = 1 + self.graphemes.count_consecutive("-", usize::MAX);
                        if dashes == 1 {
                            self.parse_list(i)
                        } else if dashes == 3
                            && self.graphemes.count_consecutive_by(|g| g.contains('\n'), 2) == 2
                        {
                            (BlockElement::Break, i..i + dashes + 2)
                        } else {
                            self.parse_paragraph(i, Alignment::Left)
                        }
                    }
                    "$" => {
                        if let Some((_, "$")) = self.graphemes.next() {
                            self.parse_math(i)
                        } else {
                            self.parse_paragraph(i, Alignment::Left)
                        }
                    }
                    _ => self.parse_paragraph(i, Alignment::Left),
                };
                return Some((block, range));
            } else {
                return Some(self.parse_paragraph(i, Alignment::Left));
            }
        }

        None
    }
}

#[derive(Debug)]
struct ListItems<'a> {
    text: &'a str,
    graphemes: utils::PeekableGraphemesPrevious<'a>,
    start: usize,
}

impl<'a> ListItems<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            graphemes: utils::PeekableGraphemesPrevious::new(text),
            start: 0,
        }
    }
}

impl<'a> Iterator for ListItems<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.text.len() {
            return None;
        }

        let (end, next_start) = match self
            .graphemes
            .find_pattern_by(|prev, next| prev.contains('\n') && next == "-")
        {
            Some((i, p, n)) => (i - p.len(), i + n.len()),
            None => (self.text.len(), self.text.len()),
        };

        let item = self.text[self.start..end].trim();
        self.start = next_start;
        Some(item)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SyntaxHighlightTheme {
    Base16OceanDark,
    Base16OceanLight,
    Base16MochaDark,
    Base16EightiesDark,
    InspiredGitHub,
    SolarizedDark,
    SolarizedLight,
}

impl SyntaxHighlightTheme {
    const fn as_str(self) -> &'static str {
        match self {
            SyntaxHighlightTheme::Base16OceanDark => "base16-ocean.dark",
            SyntaxHighlightTheme::Base16OceanLight => "base16-ocean.light",
            SyntaxHighlightTheme::Base16MochaDark => "base16-mocha.dark",
            SyntaxHighlightTheme::Base16EightiesDark => "base16-eighties.dark",
            SyntaxHighlightTheme::InspiredGitHub => "InspiredGitHub",
            SyntaxHighlightTheme::SolarizedDark => "Solarized (dark)",
            SyntaxHighlightTheme::SolarizedLight => "Solarized (light)",
        }
    }
}

struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl CodeHighlighter {
    fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn highlight(
        &self,
        language: &str,
        code: &str,
        theme: SyntaxHighlightTheme,
        mut f: impl FnMut(&str, Option<(u8, u8, u8)>),
    ) {
        let syntax = if language.is_empty() {
            self.syntax_set.find_syntax_plain_text()
        } else {
            self.syntax_set
                .find_syntax_by_token(language)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        let mut highlighter = HighlightLines::new(syntax, &self.theme_set.themes[theme.as_str()]);
        for code_line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(code_line, &self.syntax_set) {
                Ok(spans) => {
                    for (style, span) in spans {
                        let syntect::highlighting::Color { r, g, b, .. } = style.foreground;
                        f(span, Some((r, g, b)));
                    }
                }
                Err(_) => {
                    f(code_line, None);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InlineEvent<'a> {
    Text(&'a str),
    Tag(InlineTag),
}

#[derive(Debug, Clone, Copy)]
enum InlineTag {
    BoldStart,
    BoldEnd,
    ItalicStart,
    ItalicEnd,
}

impl InlineTag {
    const fn into_ansi(self) -> AnsiTag {
        match self {
            InlineTag::BoldStart => AnsiTag::Bold,
            InlineTag::BoldEnd => AnsiTag::NotBold,
            InlineTag::ItalicStart => AnsiTag::Italic,
            InlineTag::ItalicEnd => AnsiTag::NotItalic,
        }
    }
}

struct InlineParser<'a> {
    input: &'a str,
    chars: utils::PeekableCharsPrevious<'a>,
    start: usize,
    tag: Option<InlineTag>,
    bold: bool,
    italic: bool,
}

impl<'a> InlineParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: utils::PeekableCharsPrevious::new(input),
            start: 0,
            tag: None,
            bold: false,
            italic: false,
        }
    }
}

impl<'a> Iterator for InlineParser<'a> {
    type Item = InlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(tag) = self.tag.take() {
            return Some(InlineEvent::Tag(tag));
        }

        /// Looks for valid inline tag from the current, previous and next chars.
        /// If valid, either a start tag or end tag is returned based on the tag condition.
        /// That is, if tag condition is true, then the end tag will be returned,
        /// since this implies that a start tag has been found earlier.
        fn parse_tag(
            curr: char,
            prev: Option<char>,
            next: Option<char>,
            tag_start: InlineTag,
            tag_end: InlineTag,
            tag_cond: &mut bool,
        ) -> Option<InlineTag> {
            match (prev, next) {
                // Look for both tags based on tag condition
                (Some(prev), Some(next)) => {
                    if *tag_cond {
                        if !prev.is_whitespace() && prev != curr {
                            *tag_cond = false;
                            return Some(tag_end);
                        }
                    } else {
                        if !next.is_whitespace() && next != curr {
                            *tag_cond = true;
                            return Some(tag_start);
                        }
                    }
                }
                // First char, only need to look for start tag
                (None, Some(next)) => {
                    if !*tag_cond {
                        if !next.is_whitespace() && next != curr {
                            *tag_cond = true;
                            return Some(tag_start);
                        }
                    }
                }
                // Last char, only need to look for end tag
                (Some(prev), None) => {
                    if *tag_cond {
                        if !prev.is_whitespace() && prev != curr {
                            *tag_cond = false;
                            return Some(tag_end);
                        }
                    }
                }
                // Nothing to look for
                (None, None) => {}
            }

            None
        }

        while let Some((i, c)) = self.chars.next() {
            let tag = match c {
                '*' => parse_tag(
                    c,
                    self.chars.previous(),
                    self.chars.peek(),
                    InlineTag::BoldStart,
                    InlineTag::BoldEnd,
                    &mut self.bold,
                ),
                '_' => parse_tag(
                    c,
                    self.chars.previous(),
                    self.chars.peek(),
                    InlineTag::ItalicStart,
                    InlineTag::ItalicEnd,
                    &mut self.italic,
                ),
                _ => None,
            };

            if let Some(tag) = tag {
                let text = &self.input[self.start..i];
                self.start = i + c.len_utf8();
                if text.is_empty() {
                    return Some(InlineEvent::Tag(tag));
                } else {
                    self.tag = Some(tag);
                    return Some(InlineEvent::Text(text));
                }
            }
        }

        let remaining = &self.input[self.start..];
        if remaining.is_empty() {
            None
        } else {
            self.start = self.input.len();
            Some(InlineEvent::Text(remaining))
        }
    }
}
