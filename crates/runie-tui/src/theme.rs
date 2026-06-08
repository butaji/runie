//! Design System — tui1-inspired theme
//!
//! Color palette:
//! - bg:       "#0c0c0c" (dark background)
//! - fg:       "#4a4a4a" (default/dim text)
//! - fgMid:    "#6a6a6a" (mid text)
//! - fgBright: "#909090" (bright text)
//! - accent:   "#8b7cf4" (purple accent - thinking)
//! - success:  "#3ebd6a" (green - working)
//! - warning:  "#eab84a" (yellow - warnings)
//! - dim:      "#282828" (very dim UI chrome)

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
}

impl Colors {
    pub const fn new() -> Self {
        Self {
            bg: Color::Rgb(12, 12, 12),
            fg: Color::Rgb(74, 74, 74),
            fg_mid: Color::Rgb(106, 106, 106),
            fg_bright: Color::Rgb(144, 144, 144),
            accent: Color::Rgb(139, 124, 244),
            success: Color::Rgb(62, 189, 106),
            warning: Color::Rgb(234, 184, 74),
            dim: Color::Rgb(40, 40, 40),
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
