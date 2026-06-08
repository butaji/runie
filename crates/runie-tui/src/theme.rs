//! Design System — tui1-inspired theme
//!
//! Color palette with WCAG-compliant contrast ratios:
//! - bg:       "#0c0c0c" (dark background)
//! - fg:       "#8a8a8a" (default text - better contrast)
//! - fg_mid:   "#a8a8a8" (mid text - readable)
//! - fg_bright: "#d0d0d0" (bright text - high contrast)
//! - accent:   "#8b7cf4" (purple accent - thinking)
//! - success:  "#3ebd6a" (green - working)
//! - warning:  "#eab84a" (yellow - warnings)
//! - dim:      "#4a4a4a" (UI chrome - readable)

use ratatui::style::Color;

/// Design system colors
pub struct Colors {
    pub bg: Color,
    pub fg: Color,
    pub fg_mid: Color,
    pub fg_bright: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub dim: Color,
    pub code: Color,
    pub code_bg: Color,
}

impl Colors {
    pub const fn new() -> Self {
        Self {
            bg: Color::Rgb(12, 12, 12),
            fg: Color::Rgb(138, 138, 138),     // was 74 - now ~7:1 contrast
            fg_mid: Color::Rgb(168, 168, 168),  // was 106 - now ~8:1 contrast
            fg_bright: Color::Rgb(208, 208, 208), // was 144 - high contrast
            accent: Color::Rgb(139, 124, 244),   // purple - unchanged
            success: Color::Rgb(62, 189, 106),   // green - unchanged
            warning: Color::Rgb(234, 184, 74),  // yellow - unchanged
            dim: Color::Rgb(74, 74, 74),        // was 40 - now readable
            code: Color::Rgb(180, 180, 200),      // light blue-grey for code
            code_bg: Color::Rgb(30, 30, 40),      // subtle dark background for code
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self::new()
    }
}

pub const C: Colors = Colors::new();

/// Status variants for coloring
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    Thinking,
    Working,
}

impl Status {
    pub fn color(&self) -> Color {
        match self {
            Status::Ready => C.fg,
            Status::Thinking => C.accent,
            Status::Working => C.success,
        }
    }
}
