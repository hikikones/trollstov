use ratatui::{buffer::Buffer, layout::Rect, style::Color};

pub struct LogoWidget;

impl LogoWidget {
    pub fn render(area: Rect, buf: &mut Buffer) {
        let (cx, cy) = (area.width / 2, area.height / 2);
        let radius = 10;
        let color = Color::Yellow;

        draw_circle(buf, cx, cy, radius, color);

        let ray_count = 12;
        let ray_length = radius / 2;
        let ray_offset = radius / 4;

        for i in 0..ray_count {
            let angle = i as f32 * (2.0 * std::f32::consts::PI / ray_count as f32);

            let inner_r = (radius + ray_offset) as f32;
            let outer_r = (radius + ray_offset + ray_length) as f32;

            let x1 = cx as f32 + inner_r * angle.cos() * 2.0;
            let y1 = cy as f32 + inner_r * angle.sin();

            let x2 = cx as f32 + outer_r * angle.cos() * 2.0;
            let y2 = cy as f32 + outer_r * angle.sin();

            let p1 = (x1.round() as u16, y1.round() as u16);
            let p2 = (x2.round() as u16, y2.round() as u16);

            draw_line(p1, p2, color, buf);
        }

        super::utils::print_ascii(
            area,
            buf,
            "SOLBYTE",
            color.into(),
            super::utils::Alignment::Center,
        );
    }
}

pub fn draw_circle(buf: &mut Buffer, cx: u16, cy: u16, radius: u16, color: Color) {
    let mut x = radius as i16;
    let mut y = 0;
    let mut err = 0;

    let cx = cx as i16;
    let cy = cy as i16;

    while x >= y {
        let points = [
            (cx + x * 2, cy + y),
            (cx + x * 2 - 1, cy + y),
            (cx + y * 2, cy + x),
            (cx + y * 2 - 1, cy + x),
            (cx - y * 2, cy + x),
            (cx - y * 2 + 1, cy + x),
            (cx - x * 2, cy + y),
            (cx - x * 2 + 1, cy + y),
            (cx - x * 2, cy - y),
            (cx - x * 2 + 1, cy - y),
            (cx - y * 2, cy - x),
            (cx - y * 2 + 1, cy - x),
            (cx + y * 2, cy - x),
            (cx + y * 2 - 1, cy - x),
            (cx + x * 2, cy - y),
            (cx + x * 2 - 1, cy - y),
        ];

        for (px, py) in points {
            if px >= 0
                && py >= 0
                && let Some(cell) = buf.cell_mut((px as u16, py as u16))
            {
                cell.set_bg(color);
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

fn draw_line(p1: (u16, u16), p2: (u16, u16), color: Color, buf: &mut Buffer) {
    let (mut x1, mut y1) = (p1.0 as i16, p1.1 as i16);
    let (x2, y2) = (p2.0 as i16, p2.1 as i16);

    let dx = (x2 - x1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let dy = -(y2 - y1).abs();
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if let Some(cell) = buf.cell_mut((x1 as u16, y1 as u16)) {
            cell.set_bg(color);
        }

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
