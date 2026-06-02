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
