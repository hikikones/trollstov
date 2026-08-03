use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Color,
    widgets::{Block, Widget},
};
use shared::terminal::TerminalCellSize;

pub struct Image {
    id: u32,
    encoded: String,
    dims: Dimensions,
    resize: ResizeMode,
    alignment: crate::utils::Alignment,
    generation: u32,
}

impl Image {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            encoded: String::new(),
            dims: Dimensions::ZERO,
            resize: ResizeMode::Fit,
            alignment: crate::utils::Alignment::Center,
            generation: 0,
        }
    }

    pub const fn with_resize(mut self, resize: ResizeMode) -> Self {
        self.resize = resize;
        self
    }

    pub const fn with_alignment(mut self, alignment: crate::utils::Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub const fn with_layout(
        mut self,
        resize: ResizeMode,
        alignment: crate::utils::Alignment,
    ) -> Self {
        self.resize = resize;
        self.alignment = alignment;
        self
    }

    pub const fn dims(&self) -> Dimensions {
        self.dims
    }

    pub const fn set_resize(&mut self, resize: ResizeMode) {
        self.resize = resize;
    }

    fn clear(&mut self) {
        self.encoded.clear();
        self.dims = Dimensions::ZERO;
        self.generation = 0;
    }
}

const KITTY_START: &str = "\x1b_G";
const KITTY_END: &str = "\x1b\\";

pub struct KittyGraphics {
    frames: Vec<image::Frame>,
    deflate: Deflate,
    base64: Base64,
    formatter: utils::Formatter,
    cell_size: CellSize,
    verbosity: KittyVerbosity,
    generation: u32,
}

pub enum KittyLoad<'a> {
    Path(&'a Path),
    PngBytes(&'a [u8]),
}

impl KittyGraphics {
    pub const fn new(cell_size: TerminalCellSize) -> Self {
        Self {
            frames: Vec::new(),
            deflate: Deflate::new(),
            base64: Base64::new(),
            formatter: utils::Formatter::new(),
            cell_size: CellSize::from(cell_size),
            verbosity: KittyVerbosity::Silent,
            generation: 1,
        }
    }

    pub const fn increase_generation(&mut self) {
        self.generation += 1;
    }

    pub fn load_and_encode(&mut self, from: KittyLoad, img: &mut Image) -> Result<(), KittyError> {
        match from {
            KittyLoad::Path(path) => {
                self.load_from_path(path)?;
            }
            KittyLoad::PngBytes(bytes) => {
                self.load_png_from_bytes(bytes)?;
            }
        }

        img.clear();
        img.dims = self.encode(img.id, &mut img.encoded)?;

        Ok(())
    }

    fn load_from_path(&mut self, path: &Path) -> Result<(), KittyLoadError> {
        use image::AnimationDecoder;

        let reader = image::ImageReader::open(path)?.with_guessed_format()?;

        let Some(image_format) = reader.format() else {
            let image = reader.decode()?.to_rgba8();
            self.frames.push(image::Frame::new(image));
            return Ok(());
        };

        self.frames.clear();

        match image_format {
            image::ImageFormat::Png => {
                let png_decoder = image::codecs::png::PngDecoder::new(reader.into_inner())?;
                if png_decoder.is_apng()? {
                    for frame_res in png_decoder.apng()?.into_frames() {
                        match frame_res {
                            Ok(frame) => self.frames.push(frame),
                            Err(err) => {
                                self.frames.clear();
                                return Err(err)?;
                            }
                        }
                    }
                } else {
                    let mut reader = image::ImageReader::open(path)?;
                    reader.set_format(image::ImageFormat::Png);
                    let image = reader.decode()?.to_rgba8();
                    self.frames.push(image::Frame::new(image));
                }
            }
            image::ImageFormat::Gif => {
                for frame_res in
                    image::codecs::gif::GifDecoder::new(reader.into_inner())?.into_frames()
                {
                    match frame_res {
                        Ok(frame) => self.frames.push(frame),
                        Err(err) => {
                            self.frames.clear();
                            return Err(err)?;
                        }
                    }
                }
            }
            image::ImageFormat::WebP => {
                for frame_res in
                    image::codecs::webp::WebPDecoder::new(reader.into_inner())?.into_frames()
                {
                    match frame_res {
                        Ok(frame) => self.frames.push(frame),
                        Err(err) => {
                            self.frames.clear();
                            return Err(err)?;
                        }
                    }
                }
            }
            _ => {
                let image = reader.decode()?.to_rgba8();
                self.frames.push(image::Frame::new(image));
            }
        }

        Ok(())
    }

    fn load_png_from_bytes(&mut self, bytes: &[u8]) -> Result<(), image::error::ImageError> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?;
        self.frames.clear();
        self.frames.push(image::Frame::new(image.to_rgba8()));

        Ok(())
    }

    fn encode(
        &mut self,
        id: u32,
        writer: &mut impl std::fmt::Write,
    ) -> Result<Dimensions, KittyEncodeError> {
        debug_assert_ne!(id, 0);

        let rgba = self.frames[0].buffer();
        let dims = Dimensions::from_tuple(rgba.dimensions());
        let compressed = self.deflate.compress(rgba.as_raw())?;
        let b64 = self.base64.encode(compressed);

        // Encode root image
        let (root_header, chunk_header) = self.formatter.push_fmt2(
            format_args!(
                "{},{},{},{},{},{}",
                KittyAction::Transmit,
                KittyImageFormat::Rgba32(dims),
                KittyTransfer::Direct,
                KittyId(id),
                KittyCompression::ZlibDeflate,
                self.verbosity
            ),
            format_args!("{},{}", KittyId(id), self.verbosity),
        );
        Self::write_chunks(
            self.formatter.slice(root_header),
            self.formatter.slice(chunk_header),
            b64,
            writer,
        )?;
        self.formatter.clear();

        // Animated image
        if self.frames.len() > 1 {
            for i in 1..self.frames.len() {
                let delay = self.frames[i].delay().numer_denom_ms().0 as i32;
                let rgba = self.frames[i].buffer();
                let dims = Dimensions::from_tuple(rgba.dimensions());
                let compressed = self.deflate.compress(rgba.as_raw())?;
                let b64 = self.base64.encode(compressed);

                let (root_header, chunk_header) = self.formatter.push_fmt2(
                    format_args!(
                        "{},{},{},{},{},{},{}",
                        KittyAction::AnimationTransmitFrame,
                        KittyImageFormat::Rgba32(dims),
                        KittyTransfer::Direct,
                        KittyId(id),
                        KittyAnimationGap(delay),
                        KittyCompression::ZlibDeflate,
                        self.verbosity
                    ),
                    format_args!(
                        "{},{},{}",
                        KittyAction::AnimationTransmitFrame,
                        KittyId(id),
                        self.verbosity
                    ),
                );
                Self::write_chunks(
                    self.formatter.slice(root_header),
                    self.formatter.slice(chunk_header),
                    b64,
                    writer,
                )?;
                self.formatter.clear();
            }

            // Set animation controls
            write!(
                writer,
                "{KITTY_START}{},{},{},{},{}{KITTY_END}",
                KittyAction::AnimationControl,
                KittyId(id),
                KittyAnimationState::RunNormal,
                KittyAnimationLoop::Forever,
                self.verbosity
            )?;
        }

        Ok(dims)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, image: &mut Image) {
        let area = area.intersection(buf.area);

        if area.is_empty() {
            return;
        }

        let Err(error) = self.do_render(area, image) else {
            return;
        };

        // Render error
        let block = Block::bordered()
            .title(" ERROR ")
            .title_alignment(Alignment::Center)
            .style(Color::Red);
        let inner = block.inner(area);
        block.render(area, buf);
        crate::utils::print_text(
            inner,
            buf,
            format!("{error}"),
            Color::Red,
            false,
            Some(crate::Alignment::Center),
        );
    }

    fn do_render(&self, area: Rect, image: &mut Image) -> std::io::Result<()> {
        use std::io::Write;

        let mut stdout = std::io::stdout().lock();

        // Retransmit image
        if image.generation != self.generation {
            stdout.write_all(image.encoded.as_bytes())?;
            stdout.flush()?;
            image.generation = self.generation;
        }

        let (id, dims, resize) = (image.id, image.dims, image.resize);

        // Modify image layout if necessary
        let (image_size, scale, crop) = match resize {
            ResizeMode::None => (self.area(dims), None, None),
            ResizeMode::Fit => {
                let max_dims = self.dimensions(area);
                let width_ratio = dims.width as f64 / max_dims.width as f64;
                let height_ratio = dims.height as f64 / max_dims.height as f64;

                // Scale by either columns or rows to maintain aspect ratio
                let scale = if width_ratio >= height_ratio {
                    KittyScale::Columns(area.width.min(self.columns(dims.width)))
                } else {
                    KittyScale::Rows(area.height.min(self.rows(dims.height)))
                };
                (self.area(Self::resize(dims, max_dims)), Some(scale), None)
            }
            ResizeMode::Stretch => {
                let scale = KittyScale::Stretch(area.width, area.height);
                (Area::from_rect(area), Some(scale), None)
            }
            ResizeMode::FitWidthCropHeight { rows_outside_top } => {
                let max_width = self.width(area.width);
                let resized_dims = Self::resize(dims, dims.with_width(max_width));
                let resized_area = self.area(resized_dims);

                if resized_area.rows > area.height {
                    // Need to crop height with source rectangle x, y, width, height
                    let y = self.height(rows_outside_top);
                    let height = self.height(area.height);
                    let h_ratio = dims.height as f64 / resized_dims.height as f64;
                    let scale = KittyScale::Columns(area.width.min(resized_area.columns));
                    let crop = KittyCrop {
                        x: 0,
                        y: (y as f64 * h_ratio).round() as u32,
                        width: dims.width,
                        height: (height as f64 * h_ratio).round() as u32,
                    };
                    (resized_area.with_rows(area.height), Some(scale), Some(crop))
                } else {
                    // No vertical cropping needed, just scale by width
                    let scale = KittyScale::Columns(area.width.min(self.columns(dims.width)));
                    (
                        resized_area.with_rows(self.rows(dims.height)),
                        Some(scale),
                        None,
                    )
                }
            }
        };

        // Image alignment
        let image_area = crate::utils::align(
            Rect {
                width: image_size.columns,
                height: image_size.rows,
                ..area
            },
            area,
            image.alignment,
        );

        // Set cursor position (row, col)
        let pos = image_area.as_position();
        write!(stdout, "\x1b[{};{}H", pos.y + 1, pos.x + 1)?;

        // Transmit kitty display command
        write!(
            stdout,
            "{KITTY_START}{},{},{},{},{}",
            KittyAction::Display,
            KittyId(id),
            KittyPlacement(id),
            KittyCursorMovement::NoMovement,
            self.verbosity,
        )?;

        if let Some(scale) = scale {
            write!(stdout, ",{}", scale)?;
        }

        if let Some(crop) = crop {
            write!(stdout, ",{}", crop)?;
        }

        stdout.write_all(KITTY_END.as_bytes())?;

        stdout.flush()
    }

    pub fn delete_id(&self, id: u32) -> std::io::Result<()> {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        write!(
            stdout,
            "{KITTY_START}{},{},{}{KITTY_END}",
            KittyAction::Delete(KittyDelete::Id),
            KittyId(id),
            self.verbosity
        )?;
        stdout.flush()
    }

    pub fn delete_ids<I>(&self, ids: I) -> std::io::Result<()>
    where
        I: IntoIterator<Item = u32>,
        I::IntoIter: ExactSizeIterator,
    {
        use std::io::Write;

        let ids = ids.into_iter();
        if ids.len() == 0 {
            return Ok(());
        }

        let mut stdout = std::io::stdout().lock();
        for id in ids {
            write!(
                stdout,
                "{KITTY_START}{},{},{}{KITTY_END}",
                KittyAction::Delete(KittyDelete::Id),
                KittyId(id),
                self.verbosity
            )?;
        }
        stdout.flush()
    }

    pub fn delete_range(&self, range: std::ops::RangeInclusive<u32>) -> std::io::Result<()> {
        use std::io::Write;

        let (start_id, end_id_inclusive) = range.into_inner();

        let mut stdout = std::io::stdout();
        write!(
            stdout,
            "{KITTY_START}{},{}{KITTY_END}",
            KittyAction::Delete(KittyDelete::Range(start_id, end_id_inclusive)),
            self.verbosity
        )?;
        stdout.flush()
    }

    pub fn delete_all(&self) -> std::io::Result<()> {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        write!(
            stdout,
            "{KITTY_START}{},{}{KITTY_END}",
            KittyAction::Delete(KittyDelete::AllVisible),
            self.verbosity
        )?;
        stdout.flush()
    }

    pub const fn width(&self, columns: u16) -> u32 {
        self.cell_size.width(columns)
    }

    pub const fn height(&self, rows: u16) -> u32 {
        self.cell_size.height(rows)
    }

    pub const fn columns(&self, width: u32) -> u16 {
        self.cell_size.columns(width)
    }

    pub const fn rows(&self, height: u32) -> u16 {
        self.cell_size.rows(height)
    }

    pub const fn dimensions(&self, area: Rect) -> Dimensions {
        self.cell_size.dimensions(area)
    }

    pub const fn area(&self, dims: Dimensions) -> Area {
        self.cell_size.area(dims)
    }

    pub fn resize(dims: Dimensions, max: Dimensions) -> Dimensions {
        let (rw, rh) =
            utils::resize_dimensions(dims.width, dims.height, max.width, max.height, false);
        Dimensions {
            width: rw,
            height: rh,
        }
    }

    fn write_chunks(
        root_header: &str,
        chunk_header: &str,
        b64: &str,
        writer: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
        const CHUNK_SIZE: usize = 4096;
        let b64_len = b64.len();

        if b64_len <= CHUNK_SIZE {
            return write!(writer, "{KITTY_START}{root_header};{b64}{KITTY_END}");
        }

        write!(
            writer,
            "{KITTY_START}{root_header},m=1;{}{KITTY_END}",
            &b64[0..CHUNK_SIZE]
        )?;
        let mut start = CHUNK_SIZE;
        let mut end = CHUNK_SIZE * 2;
        while end < b64_len {
            write!(
                writer,
                "{KITTY_START}{chunk_header},m=1;{}{KITTY_END}",
                &b64[start..end]
            )?;
            start = end;
            end += CHUNK_SIZE;
        }
        write!(
            writer,
            "{KITTY_START}{chunk_header},m=0;{}{KITTY_END}",
            &b64[start..]
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct CellSize {
    width: u32,
    height: u32,
}

impl CellSize {
    const fn from(cell_size: TerminalCellSize) -> Self {
        Self {
            width: cell_size.width,
            height: cell_size.height,
        }
    }

    const fn width(&self, columns: u16) -> u32 {
        columns as u32 * self.width
    }

    const fn height(&self, rows: u16) -> u32 {
        rows as u32 * self.height
    }

    const fn columns(&self, width: u32) -> u16 {
        width.div_ceil(self.width) as u16
    }

    const fn rows(&self, height: u32) -> u16 {
        height.div_ceil(self.height) as u16
    }

    const fn dimensions(&self, area: Rect) -> Dimensions {
        Dimensions {
            width: self.width(area.width),
            height: self.height(area.height),
        }
    }

    const fn area(&self, dims: Dimensions) -> Area {
        Area {
            columns: self.columns(dims.width),
            rows: self.rows(dims.height),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ResizeMode {
    None,
    #[default]
    Fit,
    Stretch,
    FitWidthCropHeight {
        rows_outside_top: u16,
    },
}

/// The area for an image.
#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub columns: u16,
    pub rows: u16,
}

impl Area {
    pub const fn new(columns: u16, rows: u16) -> Self {
        Self { columns, rows }
    }

    pub const fn from_rect(area: Rect) -> Self {
        Self::new(area.width, area.height)
    }

    pub const fn with_columns(mut self, columns: u16) -> Self {
        self.columns = columns;
        self
    }

    pub const fn with_rows(mut self, rows: u16) -> Self {
        self.rows = rows;
        self
    }
}

/// The pixel dimensions for an image.
#[derive(Debug, Clone, Copy)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub const fn from_tuple(wh: (u32, u32)) -> Self {
        Self {
            width: wh.0,
            height: wh.1,
        }
    }

    pub const fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    pub const fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyImageFormat {
    // Rgb24(Dimensions),
    Rgba32(Dimensions),
    // Png,
}

impl std::fmt::Display for KittyImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rgba32(dims) => {
                f.write_fmt(format_args!("f=32,s={},v={}", dims.width, dims.height))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyTransfer {
    Direct,
    // File,
    // TempFile,
    // SharedMemory
}

impl std::fmt::Display for KittyTransfer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KittyTransfer::Direct => f.write_str("t=d"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAction {
    Transmit,
    // TransmitAndDisplay,
    // QueryTerminal,
    Display,
    Delete(KittyDelete),
    AnimationTransmitFrame,
    // AnimationComposeFrame,
    AnimationControl,
}

impl std::fmt::Display for KittyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Transmit => f.write_str("a=t"),
            Self::Display => f.write_str("a=p"),
            Self::Delete(delete) => f.write_fmt(format_args!("a=d,{}", delete)),
            Self::AnimationTransmitFrame => f.write_str("a=f"),
            Self::AnimationControl => f.write_str("a=a"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyId(u32);

impl std::fmt::Display for KittyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("i={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyPlacement(u32);

impl std::fmt::Display for KittyPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("p={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyScale {
    Columns(u16),
    Rows(u16),
    Stretch(u16, u16),
}

impl std::fmt::Display for KittyScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Columns(cols) => f.write_fmt(format_args!("c={}", cols)),
            Self::Rows(rows) => f.write_fmt(format_args!("r={}", rows)),
            Self::Stretch(cols, rows) => f.write_fmt(format_args!("c={},r={}", cols, rows)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyCrop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl std::fmt::Display for KittyCrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "x={},y={},w={},h={}",
            self.x, self.y, self.width, self.height
        ))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyDelete {
    AllVisible,
    Id,
    Range(u32, u32),
}

impl std::fmt::Display for KittyDelete {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::AllVisible => f.write_str("d=a"),
            Self::Id => f.write_str("d=i"),
            Self::Range(min, max) => f.write_fmt(format_args!("d=r,x={},y={}", min, max)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyCompression {
    ZlibDeflate,
}

impl std::fmt::Display for KittyCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZlibDeflate => f.write_str("o=z"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyCursorMovement {
    // MoveToAfterImage = 0,
    NoMovement = 1,
}

impl std::fmt::Display for KittyCursorMovement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("C={}", *self as u8))
    }
}

#[derive(Debug, Clone, Copy)]
struct KittyAnimationGap(i32);

impl std::fmt::Display for KittyAnimationGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("z={}", self.0))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAnimationState {
    // Stop = 1,
    // RunWaitLoad = 2,
    RunNormal = 3,
}

impl std::fmt::Display for KittyAnimationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("s={}", *self as u8))
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyAnimationLoop {
    // Ignore,
    Forever,
    // Amount(u32),
}

impl std::fmt::Display for KittyAnimationLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            // Self::Ignore => f.write_str("v=0"),
            Self::Forever => f.write_str("v=1"),
            // Self::Amount(n) => f.write_fmt(format_args!("v={}", n + 1)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KittyVerbosity {
    // All = 0,
    // ErrorsOnly = 1,
    Silent = 2,
}

impl std::fmt::Display for KittyVerbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("q={}", *self as u8))
    }
}

struct Deflate(Vec<u8>);

impl Deflate {
    const fn new() -> Self {
        Self(Vec::new())
    }

    fn compress(&mut self, input: &[u8]) -> Result<&[u8], DeflateError> {
        // Reset buffer with zeroes
        let zeroes = zlib_rs::compress_bound(input.len());
        self.0.clear();
        self.0.extend(std::iter::repeat_n(0, zeroes));

        // Compress
        let config = zlib_rs::DeflateConfig::default();
        let (compressed, rc) = zlib_rs::compress_slice(&mut self.0, input, config);
        match rc {
            zlib_rs::ReturnCode::Ok => Ok(compressed),
            _ => Err(DeflateError(rc)),
        }
    }
}

#[derive(Debug)]
pub struct DeflateError(zlib_rs::ReturnCode);

impl std::fmt::Display for DeflateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = unsafe { std::ffi::CStr::from_ptr(self.0.error_message()) };
        f.write_fmt(format_args!(
            "{} in deflate compression",
            error.to_string_lossy()
        ))
    }
}

impl std::error::Error for DeflateError {}

struct Base64(String); // TODO: simd

impl Base64 {
    const fn new() -> Self {
        Self(String::new())
    }

    fn encode(&mut self, input: &[u8]) -> &str {
        use base64::{Engine, engine::general_purpose::STANDARD};

        self.0.clear();
        STANDARD.encode_string(input, &mut self.0);
        self.0.as_str()
    }
}

#[derive(Debug)]
pub enum KittyLoadError {
    Io(std::io::Error),
    Image(image::error::ImageError),
}

impl std::fmt::Display for KittyLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => f.write_fmt(format_args!("std::io::Error: {err}")),
            Self::Image(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyLoadError {}

impl From<std::io::Error> for KittyLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<image::error::ImageError> for KittyLoadError {
    fn from(value: image::error::ImageError) -> Self {
        Self::Image(value)
    }
}

#[derive(Debug)]
pub enum KittyEncodeError {
    Fmt(std::fmt::Error),
    Compress(DeflateError),
}

impl std::fmt::Display for KittyEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fmt(err) => f.write_fmt(format_args!("std::fmt::Error: {err}")),
            Self::Compress(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyEncodeError {}

impl From<std::fmt::Error> for KittyEncodeError {
    fn from(value: std::fmt::Error) -> Self {
        Self::Fmt(value)
    }
}

impl From<DeflateError> for KittyEncodeError {
    fn from(value: DeflateError) -> Self {
        Self::Compress(value)
    }
}

#[derive(Debug)]
pub enum KittyError {
    Load(KittyLoadError),
    Encode(KittyEncodeError),
}

impl std::fmt::Display for KittyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(err) => err.fmt(f),
            Self::Encode(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KittyError {}

impl From<image::error::ImageError> for KittyError {
    fn from(value: image::error::ImageError) -> Self {
        Self::Load(KittyLoadError::Image(value))
    }
}

impl From<KittyLoadError> for KittyError {
    fn from(value: KittyLoadError) -> Self {
        Self::Load(value)
    }
}

impl From<KittyEncodeError> for KittyError {
    fn from(value: KittyEncodeError) -> Self {
        Self::Encode(value)
    }
}
