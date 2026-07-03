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
        const _: () = assert!(
            POPUP_WIDTH >= POPUP_MIN_WIDTH,
            "popup width must be at least min width"
        );
        const _: () = assert!(
            POPUP_HEIGHT >= POPUP_MIN_HEIGHT,
            "popup height must be at least min height"
        );
        const _: () = assert!(POPUP_WIDTH > 0, "popup width must be positive");
        const _: () = assert!(POPUP_HEIGHT > 0, "popup height must be positive");
    }

    #[test]
    fn path_display_count_is_positive() {
        const _: () = assert!(
            PATH_DISPLAY_COUNT > 0,
            "path display count must be positive"
        );
    }

    #[test]
    fn hotkey_area_height_is_positive() {
        const _: () = assert!(
            HOTKEY_AREA_HEIGHT > 0,
            "hotkey area height must be positive"
        );
    }

    #[test]
    fn constants_are_consistent() {
        // Border should be at least 2 (top + bottom)
        const _: () = assert!(
            PATH_POPUP_BORDER >= 2,
            "border must be at least 2 for top + bottom"
        );
    }
}
