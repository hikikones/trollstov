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
                accent: Color::LightBlue,
                on_accent: Color::Black,
                neutral: Color::DarkGray,
                on_neutral: Color::Gray,
            },
        }
    }

    pub(super) fn accent(mut self, accent: Option<Color>) -> Self {
        if let Some(accent) = accent {
            self.accent = accent;
            self.on_accent = perceptual_text_color(accent);
        }
        self
    }

    pub(super) fn neutral(mut self, neutral: Option<Color>) -> Self {
        if let Some(neutral) = neutral {
            self.neutral = neutral;
            self.on_neutral = perceptual_text_color(neutral);
        }
        self
    }
}

/// Returns a perceptually readable foreground color for a given background color.
///
/// This function aims to ensure **WCAG AAA contrast** (≥ 7:1) while preserving
/// perceptual characteristics. It works for ANSI colors, 256-color
/// indexed palettes, and truecolor (RGB).
fn perceptual_text_color(bg: Color) -> Color {
    const AAA_CONTRAST: f32 = 7.0;

    let bg_rgb = to_rgb(bg);
    let (l, a, b) = rgb_to_lab(bg_rgb.0, bg_rgb.1, bg_rgb.2);

    // Decide whether we need dark or light text
    let want_dark = l > 50.0;

    // Reduce chroma for readability
    let chroma_scale = 0.5;
    let base_a = a * chroma_scale;
    let base_b = b * chroma_scale;

    // Search for minimum L* that satisfies AAA
    let mut best_candidate = None;

    if want_dark {
        // search downward from background lightness
        let mut test_l = l.min(95.0);
        while test_l >= 0.0 {
            let candidate = lab_to_rgb(test_l, base_a, base_b);
            if contrast_ratio(candidate, bg_rgb) >= AAA_CONTRAST {
                best_candidate = Some(candidate);
                break;
            }
            test_l -= 2.0;
        }
    } else {
        // search upward
        let mut test_l = l.max(5.0);
        while test_l <= 100.0 {
            let candidate = lab_to_rgb(test_l, base_a, base_b);
            if contrast_ratio(candidate, bg_rgb) >= AAA_CONTRAST {
                best_candidate = Some(candidate);
                break;
            }
            test_l += 2.0;
        }
    }

    if let Some((r, g, b)) = best_candidate {
        Color::Rgb(r, g, b)
    } else {
        fallback_bw(bg_rgb)
    }
}

//////////////////////////////////////////////////////////////
// WCAG Contrast
//////////////////////////////////////////////////////////////

fn fallback_bw(bg: (u8, u8, u8)) -> Color {
    let black = (0, 0, 0);
    let white = (255, 255, 255);

    if contrast_ratio(black, bg) > contrast_ratio(white, bg) {
        Color::Black
    } else {
        Color::White
    }
}

fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);

    if l1 > l2 {
        (l1 + 0.05) / (l2 + 0.05)
    } else {
        (l2 + 0.05) / (l1 + 0.05)
    }
}

fn relative_luminance((r, g, b): (u8, u8, u8)) -> f32 {
    fn channel(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

//////////////////////////////////////////////////////////////
// ratatui Color to RGB
//////////////////////////////////////////////////////////////

fn to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => indexed_to_rgb(i),

        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::Gray => (192, 192, 192),
        Color::DarkGray => (128, 128, 128),

        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),

        Color::Reset => (0, 0, 0),
    }
}

fn indexed_to_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        0..=15 => {
            const ANSI: [(u8, u8, u8); 16] = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            ANSI[i as usize]
        }
        16..=231 => {
            let i = i - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let scale = |v| if v == 0 { 0 } else { v * 40 + 55 };
            (scale(r), scale(g), scale(b))
        }
        232..=255 => {
            let gray = 8 + (i - 232) * 10;
            (gray, gray, gray)
        }
    }
}

//////////////////////////////////////////////////////////////
// RGB to Lab
//////////////////////////////////////////////////////////////

fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (x, y, z) = rgb_to_xyz(r, g, b);
    xyz_to_lab(x, y, z)
}

fn lab_to_rgb(l: f32, a: f32, b: f32) -> (u8, u8, u8) {
    let (x, y, z) = lab_to_xyz(l, a, b);
    xyz_to_rgb(x, y, z)
}

fn rgb_to_xyz(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = srgb_to_linear(r as f32 / 255.0);
    let g = srgb_to_linear(g as f32 / 255.0);
    let b = srgb_to_linear(b as f32 / 255.0);

    (
        r * 0.4124 + g * 0.3576 + b * 0.1805,
        r * 0.2126 + g * 0.7152 + b * 0.0722,
        r * 0.0193 + g * 0.1192 + b * 0.9505,
    )
}

fn xyz_to_rgb(x: f32, y: f32, z: f32) -> (u8, u8, u8) {
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;

    (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
}

fn xyz_to_lab(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let xr = x / 0.95047;
    let yr = y / 1.0;
    let zr = z / 1.08883;

    let fx = lab_f(xr);
    let fy = lab_f(yr);
    let fz = lab_f(zr);

    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

fn lab_to_xyz(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    (
        lab_inv_f(fx) * 0.95047,
        lab_inv_f(fy) * 1.0,
        lab_inv_f(fz) * 1.08883,
    )
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };

    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn lab_f(t: f32) -> f32 {
    if t > 0.008856 {
        t.powf(1.0 / 3.0)
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

fn lab_inv_f(t: f32) -> f32 {
    let t3 = t * t * t;
    if t3 > 0.008856 {
        t3
    } else {
        (t - 16.0 / 116.0) / 7.787
    }
}
