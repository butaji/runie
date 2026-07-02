//! Centralized TUI popup layout constants.
//!
//! All popup, panel, and dialog layout dimensions are defined here.
//! This ensures consistent sizing across the TUI and makes
//! adjustments trivial.

/// Default popup width in characters.
pub const POPUP_WIDTH: u16 = 60;

/// Default popup height in lines.
pub const POPUP_HEIGHT: u16 = 18;

/// Minimum popup width in characters.
pub const POPUP_MIN_WIDTH: u16 = 20;

/// Minimum popup height in lines.
pub const POPUP_MIN_HEIGHT: u16 = 6;

/// Maximum number of path suggestions to display.
pub const PATH_DISPLAY_COUNT: u16 = 8;

/// Border offset added to popup dimensions for path suggestions.
pub const PATH_POPUP_BORDER: u16 = 4;

/// Height of the hotkey hint area at the bottom of panels.
pub const HOTKEY_AREA_HEIGHT: u16 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_dimensions_are_reasonable() {
        assert!(POPUP_WIDTH >= POPUP_MIN_WIDTH);
        assert!(POPUP_HEIGHT >= POPUP_MIN_HEIGHT);
        assert!(POPUP_WIDTH > 0);
        assert!(POPUP_HEIGHT > 0);
    }

    #[test]
    fn path_display_count_is_positive() {
        assert!(PATH_DISPLAY_COUNT > 0);
    }

    #[test]
    fn hotkey_area_height_is_positive() {
        assert!(HOTKEY_AREA_HEIGHT > 0);
    }

    #[test]
    fn constants_are_consistent() {
        // Border should be at least 2 (top + bottom)
        assert!(PATH_POPUP_BORDER >= 2);
    }
}
