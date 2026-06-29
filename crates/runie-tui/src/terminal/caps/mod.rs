//! Terminal capability detection.
//!
//! Uses `supports-color` for color level detection and `supports-hyperlinks`
//! for hyperlink support. Brand, multiplexer, mouse, clipboard, focus
//! tracking, and unicode detection remain custom heuristics over an
//! environment snapshot so they can be unit-tested without touching `std::env`.

use std::collections::HashMap;

use detect::*;

/// Maximum color depth the terminal supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorDepth {
    /// No color support.
    #[default]
    None,
    /// Basic ANSI 16 colors.
    ANSI16,
    /// 256-color palette.
    ANSI256,
    /// 24-bit truecolor.
    Truecolor,
}

/// Detected terminal capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermCaps {
    /// Maximum color depth the terminal supports.
    pub color_depth: ColorDepth,
    /// Truecolor (24-bit) support is available.
    pub truecolor: bool,
    /// Hyperlink (OSC 8) support is available.
    pub hyperlinks: bool,
    pub mouse: MouseCapability,
    pub clipboard: bool,
    pub focus_tracking: bool,
    pub unicode: bool,
    pub brand: TerminalBrand,
    pub multiplexer: MultiplexerType,
}

impl Default for TermCaps {
    fn default() -> Self {
        Self {
            color_depth: ColorDepth::None,
            truecolor: false,
            hyperlinks: false,
            mouse: MouseCapability::None,
            clipboard: false,
            focus_tracking: false,
            unicode: true,
            brand: TerminalBrand::Unknown,
            multiplexer: MultiplexerType::None,
        }
    }
}

/// Backwards-compatible alias.
#[deprecated(since = "0.2.17", note = "renamed to TermCaps")]
pub type TerminalCapabilities = TermCaps;

/// Mouse protocol support level the terminal is likely to understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseCapability {
    /// No mouse events.
    None,
    /// Legacy X11 mouse protocol (1000/1002).
    Legacy,
    /// SGR-encoded coordinates (1006).
    Sgr,
    /// SGR + alternative coordinate encoding (1006 + 1015).
    SgrExtended,
}

/// Recognized terminal emulator families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalBrand {
    Unknown,
    ITerm2,
    VSCode,
    WezTerm,
    Alacritty,
    Kitty,
    TerminalApp, // macOS Terminal.app
    WindowsTerminal,
    Warp,
    Ghostty,
}

/// Recognized terminal multiplexer families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplexerType {
    None,
    Tmux,
    Screen,
    Zellij,
}

/// Detect capabilities from the current process environment.
pub fn detect_capabilities_from_env() -> TermCaps {
    let env: HashMap<String, String> = std::env::vars().collect();
    detect_capabilities(&env)
}

/// Detect capabilities from an explicit environment snapshot.
pub fn detect_capabilities(env: &HashMap<String, String>) -> TermCaps {
    let brand = detect_brand(env);
    let multiplexer = detect_multiplexer(env);
    let color_depth = detect_color_depth(env, brand, multiplexer);
    let hyperlinks = detect_hyperlinks(env);

    let truecolor = matches!(color_depth, ColorDepth::Truecolor);

    TermCaps {
        color_depth,
        truecolor,
        hyperlinks,
        mouse: detect_mouse(brand, multiplexer),
        clipboard: detect_clipboard(brand, multiplexer),
        focus_tracking: detect_focus_tracking(brand, multiplexer),
        unicode: detect_unicode(env),
        brand,
        multiplexer,
    }
}

mod detect;
#[cfg(test)]
mod tests;
