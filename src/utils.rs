use ratatui::layout::Rect;

/// Centers the inner [Rect] both horizontally and vertically inside outer [Rect].
pub fn center(inner: Rect, outer: Rect) -> Rect {
    Rect {
        x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
        y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
        width: inner.width,
        height: inner.height,
    }
}

/// Centers the inner [Rect] horizontally inside outer [Rect].
pub fn center_horizontally(inner: Rect, outer: Rect) -> Rect {
    Rect {
        x: outer.x + (outer.width.saturating_sub(inner.width)) / 2,
        y: outer.y,
        width: inner.width,
        height: inner.height,
    }
}

/// Centers the inner [Rect] vertically inside outer [Rect].
pub fn center_vertically(inner: Rect, outer: Rect) -> Rect {
    Rect {
        x: outer.x,
        y: outer.y + (outer.height.saturating_sub(inner.height)) / 2,
        width: inner.width,
        height: inner.height,
    }
}
