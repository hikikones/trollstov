use ratatui::{buffer::Buffer, layout::Rect, style::Color, text::Text, widgets::Widget};

pub struct LogoWidget;

impl Widget for LogoWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let logo = Logo::from_rect(area);
        let ascii = logo.ascii();
        let color = Color::DarkGray;
        let (width, height) = logo.dim();

        Text::styled(ascii, color).render(
            super::utils::align(
                Rect {
                    width: width,
                    height: height,
                    ..area
                },
                area,
                super::utils::Alignment::Center,
            ),
            buf,
        );
    }
}

pub struct LogoSunWidget;

impl Widget for LogoSunWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let sun_color = Color::LightYellow;
        let ray_color = Color::Yellow;

        let logo = Logo::from_rect(area);
        let radius = logo.radius();

        // Draw top half circle
        let center = (area.width / 2, area.height / 2);
        circle_points(center, radius, |p| {
            if p.1 < center.1
                && let Some(cell) = buf.cell_mut(p)
            {
                cell.set_bg(sun_color);
            }
        });

        // Draw rays
        let ray_count = 14;
        let ray_length = radius / 2;
        let ray_offset = radius / 4;
        ray_lines(
            center,
            radius,
            ray_count,
            ray_length,
            ray_offset,
            |p1, p2| {
                if p2.1 < center.1 {
                    line_points(p1, p2, |p| {
                        if let Some(cell) = buf.cell_mut(p) {
                            cell.set_bg(ray_color);
                        }
                    });
                }
            },
        );

        // Draw sunset line
        let start = center.0.saturating_sub(radius * 3 + ray_length);
        let end = center.0 + (radius * 3 + ray_length);
        line_points((start, center.1), (end, center.1), |p| {
            if let Some(cell) = buf.cell_mut(p) {
                cell.set_bg(ray_color);
            }
        });

        // Draw title
        let ascii = logo.ascii();
        let (width, height) = logo.dim();
        Text::styled(ascii, ray_color).render(
            super::utils::align(
                Rect {
                    width: width,
                    height: height,
                    y: center.1 + 1,
                    ..area
                },
                area,
                super::utils::Alignment::CenterHorizontal,
            ),
            buf,
        );
    }
}

fn circle_points(center: (u16, u16), radius: u16, mut f: impl FnMut((u16, u16))) {
    let (cx, cy) = (center.0 as i16, center.1 as i16);
    let x_scale = 2;

    let mut x = radius as i16;
    let mut y = 0;
    let mut err = 0;

    while x >= y {
        let points = [
            (cx + x * x_scale, cy + y),
            (cx + x * x_scale - 1, cy + y),
            (cx + y * x_scale, cy + x),
            (cx + y * x_scale - 1, cy + x),
            (cx - y * x_scale, cy + x),
            (cx - y * x_scale + 1, cy + x),
            (cx - x * x_scale, cy + y),
            (cx - x * x_scale + 1, cy + y),
            (cx - x * x_scale, cy - y),
            (cx - x * x_scale + 1, cy - y),
            (cx - y * x_scale, cy - x),
            (cx - y * x_scale + 1, cy - x),
            (cx + y * x_scale, cy - x),
            (cx + y * x_scale - 1, cy - x),
            (cx + x * x_scale, cy - y),
            (cx + x * x_scale - 1, cy - y),
        ];

        for (px, py) in points {
            if px >= 0 && py >= 0 {
                f((px as u16, py as u16));
            }
        }

        y += 1;
        if err <= 0 {
            err += 2 * y + 1;
        }
        if err > 0 {
            x -= 1;
            err -= 2 * x + 1;
        }
    }
}

fn ray_lines(
    center: (u16, u16),
    circle_radius: u16,
    ray_count: u8,
    ray_length: u16,
    ray_offset: u16,
    mut f: impl FnMut((u16, u16), (u16, u16)),
) {
    let (cx, cy) = (center.0 as i16, center.1 as i16);
    let radius = circle_radius as f32;
    let length = ray_length as f32;
    let offset = ray_offset as f32;
    let x_scale = 2.0;

    for i in 0..ray_count {
        let angle = i as f32 * (2.0 * std::f32::consts::PI / ray_count as f32);

        let inner_r = radius + offset;
        let outer_r = radius + offset + length;

        let x1 = cx as f32 + inner_r * angle.cos() * x_scale;
        let y1 = cy as f32 + inner_r * angle.sin();

        let x2 = cx as f32 + outer_r * angle.cos() * x_scale;
        let y2 = cy as f32 + outer_r * angle.sin();

        let p1 = (x1.round() as u16, y1.round() as u16);
        let p2 = (x2.round() as u16, y2.round() as u16);

        f(p1, p2)
    }
}

fn line_points(p1: (u16, u16), p2: (u16, u16), mut f: impl FnMut((u16, u16))) {
    let (mut x1, mut y1) = (p1.0 as i16, p1.1 as i16);
    let (x2, y2) = (p2.0 as i16, p2.1 as i16);

    let dx = (x2 - x1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let dy = -(y2 - y1).abs();
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        f((x1 as u16, y1 as u16));

        if x1 == x2 && y1 == y2 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x1 += sx;
        }
        if e2 <= dx {
            err += dx;
            y1 += sy;
        }
    }
}

enum Logo {
    Tiny,
    Small,
    Medium,
    Large,
}

impl Logo {
    const LOGO_TINY_WIDTH: u16 = 9;
    const LOGO_TINY_HEIGHT: u16 = 1;
    const LOGO_TINY: &str = "TROLLSTOV";

    const LOGO_SMALL_WIDTH: u16 = 28;
    const LOGO_SMALL_HEIGHT: u16 = 5;
    const LOGO_SMALL: &str = r#"
┌┬┐┬─┐┌─┐┬  ┬  ┌─┐┌┬┐┌─┐┬  ┬
 │ ├┬┘│ ││  │  └─┐ │ │ │└┐┌┘
 ┴ ┴└─└─┘┴─┘┴─┘└─┘ ┴ └─┘ └┘ 
"#;

    const LOGO_MEDIUM_WIDTH: u16 = 66;
    const LOGO_MEDIUM_HEIGHT: u16 = 6;
    const LOGO_MEDIUM: &str = r#"
888888 88""Yb  dP"Yb  88     88     .dP"Y8 888888  dP"Yb  Yb    dP
  88   88__dP dP   Yb 88     88     `Ybo."   88   dP   Yb  Yb  dP 
  88   88"Yb  Yb   dP 88  .o 88  .o o.`Y8b   88   Yb   dP   YbdP  
  88   88  Yb  YbodP  88ood8 88ood8 8bodP'   88    YbodP     YP   
"#;

    const LOGO_BIG_WIDTH: u16 = 98;
    const LOGO_BIG_HEIGHT: u16 = 9;
    const LOGO_BIG: &str = r#"
::::::::::: :::::::::   ::::::::  :::        :::        :::::::: ::::::::::: ::::::::  :::     :::
    :+:     :+:    :+: :+:    :+: :+:        :+:       :+:    :+:    :+:    :+:    :+: :+:     :+:
    +:+     +:+    +:+ +:+    +:+ +:+        +:+       +:+           +:+    +:+    +:+ +:+     +:+
    +#+     +#++:++#:  +#+    +:+ +#+        +#+       +#++:++#++    +#+    +#+    +:+ +#+     +:+
    +#+     +#+    +#+ +#+    +#+ +#+        +#+              +#+    +#+    +#+    +#+  +#+   +#+ 
    #+#     #+#    #+# #+#    #+# #+#        #+#       #+#    #+#    #+#    #+#    #+#   #+#+#+#  
    ###     ###    ###  ########  ########## ########## ########     ###     ########      ###    
"#;

    const fn from_rect(area: Rect) -> Self {
        if area.width > Self::LOGO_BIG_WIDTH + 12 && area.height > Self::LOGO_BIG_HEIGHT + 3 {
            Self::Large
        } else if area.width > Self::LOGO_MEDIUM_WIDTH + 8
            && area.height > Self::LOGO_MEDIUM_HEIGHT + 2
        {
            Self::Medium
        } else if area.width > Self::LOGO_SMALL_WIDTH + 4
            && area.height > Self::LOGO_SMALL_HEIGHT + 1
        {
            Self::Small
        } else {
            Self::Tiny
        }
    }

    const fn radius(&self) -> u16 {
        match self {
            Self::Tiny => 1,
            Self::Small => 2,
            Self::Medium => 4,
            Self::Large => 8,
        }
    }

    const fn dim(&self) -> (u16, u16) {
        match self {
            Self::Tiny => (Self::LOGO_TINY_WIDTH, Self::LOGO_TINY_HEIGHT),
            Self::Small => (Self::LOGO_SMALL_WIDTH, Self::LOGO_SMALL_HEIGHT),
            Self::Medium => (Self::LOGO_MEDIUM_WIDTH, Self::LOGO_MEDIUM_HEIGHT),
            Self::Large => (Self::LOGO_BIG_WIDTH, Self::LOGO_BIG_HEIGHT),
        }
    }

    const fn ascii(&self) -> &'static str {
        match self {
            Self::Tiny => Self::LOGO_TINY,
            Self::Small => Self::LOGO_SMALL,
            Self::Medium => Self::LOGO_MEDIUM,
            Self::Large => Self::LOGO_BIG,
        }
    }
}
