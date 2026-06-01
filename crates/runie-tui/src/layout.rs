use ratatui::layout::Rect;

/// Compute a centered sub-rectangle within `area` with given width and height.
pub fn centered_rect(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(w) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(h) / 2);
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}
