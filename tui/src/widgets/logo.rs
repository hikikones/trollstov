use ratatui::{buffer::Buffer, layout::Rect, style::Color};

pub struct LogoWidget;

impl LogoWidget {
    pub fn render(area: Rect, buf: &mut Buffer) {
        let center = (area.width / 2, area.height / 2);
        let radius = 10;
        let color = Color::Yellow;

        circle_points(center, radius, |p| {
            if p.1 < center.1
                && let Some(cell) = buf.cell_mut(p)
            {
                cell.set_bg(color);
            }
        });

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
                            cell.set_bg(color);
                        }
                    });
                }
            },
        );

        super::utils::print_ascii(
            area,
            buf,
            "SOLBYTE",
            color.into(),
            super::utils::Alignment::Center,
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
