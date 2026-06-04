//! Helper functions for layout calculations.

use ratatui::layout::Rect;

use super::layout;

/// Apply standard padding to an area.
///
/// Returns a new Rect with padding subtracted from each side.
///
/// # Example
///
/// ```
/// use ratatui::layout::Rect;
/// use crate::style::helpers::padded_area;
///
/// let area = Rect::new(0, 0, 20, 10);
/// let padded = padded_area(area);
/// // padded.x = 2, padded.y = 1, padded.width = 16, padded.height = 8
/// ```
pub fn padded_area(area: Rect) -> Rect {
    Rect {
        x: area.x + layout::PADDING_X,
        y: area.y + layout::PADDING_Y,
        width: area.width.saturating_sub(layout::PADDING_WIDTH),
        height: area.height.saturating_sub(layout::PADDING_HEIGHT),
    }
}

/// Calculate content width within a padded area.
pub fn content_width(area: Rect) -> u16 {
    area.width.saturating_sub(layout::PADDING_X)
}

/// Calculate message text width accounting for indent and markers.
pub fn message_text_width(area: Rect, indent: u16) -> u16 {
    area.width.saturating_sub(indent + 4)
}

/// Calculate the inner width of a panel after subtracting border widths.
pub fn inner_width(area: Rect) -> u16 {
    area.width.saturating_sub(2)
}

/// Calculate the inner height of a panel after subtracting border heights.
pub fn inner_height(area: Rect) -> u16 {
    area.height.saturating_sub(2)
}

/// Center a rectangle within another rectangle horizontally.
pub fn center_x(outer: Rect, inner_width: u16) -> u16 {
    outer.x + (outer.width.saturating_sub(inner_width)) / 2
}

/// Center a rectangle within another rectangle vertically.
pub fn center_y(outer: Rect, inner_height: u16) -> u16 {
    outer.y + (outer.height.saturating_sub(inner_height)) / 2
}

/// Check if a point is within a rectangle.
pub fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

/// Split an area into left and right panels with a separator.
pub fn split_with_separator(area: Rect, left_width: u16) -> (Rect, Rect) {
    let separator_width = 1u16;
    let left = Rect::new(area.x, area.y, left_width, area.height);
    let right = Rect::new(
        area.x + left_width + separator_width,
        area.y,
        area.width.saturating_sub(left_width + separator_width),
        area.height,
    );
    (left, right)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn test_padded_area() {
        let area = Rect::new(0, 0, 20, 10);
        let padded = padded_area(area);
        assert_eq!(padded.x, 2);
        assert_eq!(padded.y, 1);
        assert_eq!(padded.width, 16);
        assert_eq!(padded.height, 8);
    }

    #[test]
    fn test_content_width() {
        let area = Rect::new(0, 0, 20, 10);
        assert_eq!(content_width(area), 18);
    }

    #[test]
    fn test_message_text_width() {
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(message_text_width(area, 3), 33);
    }

    #[test]
    fn test_center_x() {
        let outer = Rect::new(0, 0, 100, 20);
        assert_eq!(center_x(outer, 20), 40);
    }

    #[test]
    fn test_center_y() {
        let outer = Rect::new(0, 0, 100, 20);
        assert_eq!(center_y(outer, 10), 5);
    }

    #[test]
    fn test_contains() {
        let area = Rect::new(5, 5, 10, 10);
        assert!(contains(area, 5, 5));
        assert!(contains(area, 10, 10));
        assert!(!contains(area, 4, 5));
        assert!(!contains(area, 15, 5));
    }

    #[test]
    fn test_split_with_separator() {
        let area = Rect::new(0, 0, 50, 20);
        let (left, right) = split_with_separator(area, 20);
        assert_eq!(left.width, 20);
        assert_eq!(right.x, 21);
        assert_eq!(right.width, 29);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Wireframe debugger (enabled with `RUNIE_WIREFRAME=1` env var)
// ─────────────────────────────────────────────────────────────────────
//
// Drop into any render path to overlay ASCII box borders + size labels
// on top of your real UI. Lets you see exactly where Constraint
// rectangles landed without rebuilding.
//
// Usage:
//   RUNIE_WIREFRAME=1 runie                    # overlay everything
//   RUNIE_WIREFRAME=ui runie                   # overlay just main layout
//   RUNIE_WIREFRAME=input,top_bar runie        # overlay specific components

/// Returns true if the wireframe overlay is enabled for the given
/// component name. `all` matches every component.
pub fn wireframe_enabled(component: &str) -> bool {
    wireframe_enabled_for(component, &std::env::var("RUNIE_WIREFRAME").ok())
}

/// Variant of [`wireframe_enabled`] that takes the env value directly.
/// Use this in tests so the env-var race (multiple threads modifying
/// `RUNIE_WIREFRAME` concurrently) is sidestepped.
pub fn wireframe_enabled_for(component: &str, env: &Option<String>) -> bool {
    match env {
        Some(v) if !v.is_empty() => v
            .split(',')
            .map(|s| s.trim())
            .any(|f| f == "all" || f == component),
        _ => false,
    }
}

/// Draw a labeled box at `area` showing its position and size. Cheap to
/// call; does nothing when wireframe mode is off.
pub fn wireframe_box(buf: &mut ratatui::buffer::Buffer, component: &str, area: Rect) {
    wireframe_box_for(buf, component, area, &std::env::var("RUNIE_WIREFRAME").ok());
}

/// Test-friendly variant that takes the env value directly. Avoids the
/// multithreaded env-var race that `std::env::set_var` triggers in
/// parallel test runs.
pub fn wireframe_box_for(
    buf: &mut ratatui::buffer::Buffer,
    component: &str,
    area: Rect,
    env: &Option<String>,
) {
    if !wireframe_enabled_for(component, env) {
        return;
    }
    use ratatui::{
        style::{Color, Style},
        widgets::{Block, Borders, Widget},
    };
    let title = format!(" {} {}x{} @({},{}) ", component, area.width, area.height, area.x, area.y);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title);
    (&block).render(area, buf);
}
