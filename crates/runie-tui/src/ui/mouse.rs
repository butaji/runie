//! Mouse hit-testing: convert terminal coordinates to UI regions.
//!
//! The render actor knows the exact layout of every frame, so it is the
//! authoritative place to compute `MouseTarget`. The core only tracks raw
//! `(row, col)` positions; the TUI translates those into semantic regions.

use runie_core::Snapshot;

/// Which region of the TUI the mouse is currently over.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MouseTarget {
    /// Mouse is over the scrollable message feed.
    Feed,
    /// Mouse is over the input box area.
    Input,
    /// Mouse is over the status bar.
    StatusBar,
    /// Mouse is over the hints line.
    Hints,
    /// No mouse tracking available or position unknown.
    #[default]
    Unknown,
}

/// Derive the mouse target from a snapshot's raw mouse position and layout.
///
/// The layout mirrors `draw_snapshot` exactly:
///   margin(1) + feed + margin(1) + status(1) + input + margin(1) + hints(1)
/// All coordinates are 0-based.
pub fn compute_mouse_target(snap: &Snapshot) -> MouseTarget {
    let (row, col) = match snap.mouse_position {
        Some(pos) => pos,
        None => return MouseTarget::Unknown,
    };

    let (width, height) = (snap.content_width, snap.last_visible_height);

    // Replicate the layout math from draw_snapshot:
    let margin = if width > 20 && height > 10 { 1 } else { 0 };
    let area_height = height.saturating_sub(margin * 2);
    let input_lines = snap.input.lines().count().max(1);
    let input_height = (input_lines + 2).min(10) as u16;

    // 1 row margin + feed + 1 row margin + 1 row status + input + 1 row margin + 1 row hints
    let feed_end = margin + area_height.saturating_sub(input_height + 4);

    if row < margin || col > width || width == 0 || height == 0 {
        MouseTarget::Unknown
    } else if row < feed_end {
        MouseTarget::Feed
    } else {
        let mut y = feed_end + margin; // skip second margin
        y += 1; // status bar
        if row < y {
            MouseTarget::StatusBar
        } else {
            y += input_height;
            if row < y {
                MouseTarget::Input
            } else {
                MouseTarget::Hints
            }
        }
    }
}

/// Returns true if the given mouse row/col falls within the message feed area.
pub fn is_in_feed(snap: &Snapshot) -> bool {
    compute_mouse_target(snap) == MouseTarget::Feed
}

/// Returns true if the given mouse row/col falls within the input area.
pub fn is_in_input(snap: &Snapshot) -> bool {
    compute_mouse_target(snap) == MouseTarget::Input
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::snapshot::MouseTarget as CoreMouseTarget;
    use std::sync::Arc;

    fn snap(height: u16, width: u16, row: Option<(u16, u16)>) -> Snapshot {
        Snapshot {
            content_width: width,
            last_visible_height: height,
            mouse_position: row,
            ..Default::default()
        }
    }

    fn core_target(snap: &Snapshot) -> CoreMouseTarget {
        match compute_mouse_target(snap) {
            MouseTarget::Feed => CoreMouseTarget::Feed,
            MouseTarget::Input => CoreMouseTarget::Input,
            MouseTarget::StatusBar => CoreMouseTarget::StatusBar,
            MouseTarget::Hints => CoreMouseTarget::Hints,
            MouseTarget::Unknown => CoreMouseTarget::Unknown,
        }
    }

    #[test]
    fn no_position_unknown() {
        let s = snap(24, 80, None);
        assert_eq!(core_target(&s), CoreMouseTarget::Unknown);
    }

    #[test]
    fn feed_row_is_feed() {
        // Row 5 on a 24x80 terminal with empty input → feed area
        let s = snap(24, 80, Some((5, 40)));
        assert_eq!(core_target(&s), CoreMouseTarget::Feed);
    }

    #[test]
    fn bottom_rows_are_input() {
        // Last few rows should be input
        let s = snap(24, 80, Some((20, 10)));
        assert_eq!(core_target(&s), CoreMouseTarget::Input);
    }

    #[test]
    fn status_bar_row() {
        // Status bar is roughly row 17-18 on a 24-row terminal
        let s = snap(24, 80, Some((17, 40)));
        assert_eq!(core_target(&s), CoreMouseTarget::StatusBar);
    }

    #[test]
    fn hints_row() {
        // Hints is the last row
        let s = snap(24, 80, Some((23, 10)));
        assert_eq!(core_target(&s), CoreMouseTarget::Hints);
    }

    #[test]
    fn out_of_bounds_unknown() {
        // Column way off to the right
        let s = snap(24, 80, Some((5, 200)));
        assert_eq!(core_target(&s), CoreMouseTarget::Unknown);
    }

    #[test]
    fn tiny_terminal() {
        // Tiny terminal with no margin
        let s = snap(10, 20, Some((5, 10)));
        assert_eq!(core_target(&s), CoreMouseTarget::Input);
    }
}
