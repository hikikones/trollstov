use ratatui::{layout::Rect, widgets::Padding};

pub trait RectExt {
    fn inner_padding(self, padding: Padding) -> Rect;
    fn shrink_down(&mut self, n: u16);
}

impl RectExt for Rect {
    fn inner_padding(self, padding: Padding) -> Rect {
        Rect {
            x: self.x + padding.left,
            y: self.y + padding.top,
            width: self.width.saturating_sub(padding.left + padding.right),
            height: self.height.saturating_sub(padding.top + padding.bottom),
        }
    }

    fn shrink_down(&mut self, n: u16) {
        self.height = self.height.saturating_sub(n);
        self.y += n;
    }
}
