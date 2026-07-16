//! Screen modes for the TUI (from Grok Build)

/// Screen rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenMode {
    /// Full alternate-screen TUI (default)
    #[default]
    Fullscreen,
    /// Embedded/inline mode (no alternate screen)
    Inline,
    /// Minimal mode - transcript survives app death via insert_before
    Minimal,
}

impl ScreenMode {
    /// Auto-detect best mode based on terminal capabilities
    pub fn detect() -> Self {
        if Self::supports_insert_before() {
            ScreenMode::Minimal
        } else {
            ScreenMode::Fullscreen
        }
    }

    /// Detect if terminal supports insert_before (scrollback survival)
    fn supports_insert_before() -> bool {
        std::env::var("TERM").map(|t| {
            t.contains("xterm-kitty") ||
            t.contains("wezterm") ||
            t.contains("tmux") ||
            t.contains("screen")
        }).unwrap_or(false)
    }

    /// Returns true if we should suppress rows above the prompt
    pub fn should_compact(&self, rows: u16) -> bool {
        matches!(self, ScreenMode::Minimal) && rows <= SHORT_TERMINAL_ROWS
    }

    /// Returns true if this mode uses alternate screen
    pub fn uses_alternate_screen(&self) -> bool {
        matches!(self, ScreenMode::Fullscreen)
    }

    /// Returns true if this mode preserves scrollback
    pub fn preserves_scrollback(&self) -> bool {
        matches!(self, ScreenMode::Minimal | ScreenMode::Inline)
    }

    /// Get the terminal control sequence prefix for this mode
    pub fn control_prefix(&self) -> &'static str {
        match self {
            ScreenMode::Fullscreen => "\x1b[?1049h", // Enter alternate screen
            ScreenMode::Inline => "",                  // No control needed
            ScreenMode::Minimal => "",                  // No control needed
        }
    }

    /// Get the terminal control sequence suffix for this mode
    pub fn control_suffix(&self) -> &'static str {
        match self {
            ScreenMode::Fullscreen => "\x1b[?1049l", // Exit alternate screen
            ScreenMode::Inline => "",                 // No control needed
            ScreenMode::Minimal => "",                 // No control needed
        }
    }

    /// Parse from string value
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fullscreen" | "full" | "alt" | "alternate" => Some(ScreenMode::Fullscreen),
            "inline" | "embedded" => Some(ScreenMode::Inline),
            "minimal" | "min" | "scrollback" => Some(ScreenMode::Minimal),
            "auto" | "detect" => Some(Self::detect()),
            _ => None,
        }
    }
}

impl std::fmt::Display for ScreenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenMode::Fullscreen => write!(f, "fullscreen"),
            ScreenMode::Inline => write!(f, "inline"),
            ScreenMode::Minimal => write!(f, "minimal"),
        }
    }
}

/// Terminal height threshold for compact UI
pub const SHORT_TERMINAL_ROWS: u16 = 16;
pub const AUTO_COMPACT_MAX_ROWS: u16 = 20;

/// Screen dimensions
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenSize {
    pub rows: u16,
    pub cols: u16,
}

impl ScreenSize {
    /// Get from terminal
    pub fn from_terminal() -> Self {
        // Try to get from environment first
        if let (Ok(cols), Ok(rows)) = (
            std::env::var("COLUMNS").ok().and_then(|s| s.parse().ok()),
            std::env::var("LINES").ok().and_then(|s| s.parse().ok()),
        ) {
            return Self { rows, cols };
        }

        // Fall back to termion
        #[cfg(feature = "termion")]
        {
            if let Ok((cols, rows)) = termion::terminal_size() {
                return Self { rows, cols };
            }
        }

        // Default fallback
        Self { rows: 24, cols: 80 }
    }

    /// Check if terminal is short
    pub fn is_short(&self) -> bool {
        self.rows <= SHORT_TERMINAL_ROWS
    }

    /// Check if terminal is wide
    pub fn is_wide(&self) -> bool {
        self.cols >= 120
    }

    /// Check if terminal is compact
    pub fn is_compact(&self) -> bool {
        self.rows <= AUTO_COMPACT_MAX_ROWS
    }
}

/// Compact mode settings
#[derive(Debug, Clone)]
pub struct CompactSettings {
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Truncate long lines
    pub truncate_lines: bool,
    /// Max line length when truncating
    pub max_line_length: usize,
    /// Compact file headers
    pub compact_headers: bool,
    /// Show abbreviated paths
    pub abbreviate_paths: bool,
}

impl Default for CompactSettings {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            truncate_lines: true,
            max_line_length: 100,
            compact_headers: true,
            abbreviate_paths: true,
        }
    }
}

impl CompactSettings {
    /// Settings for very small terminals (< 16 rows)
    pub fn very_compact() -> Self {
        Self {
            show_line_numbers: false,
            truncate_lines: true,
            max_line_length: 80,
            compact_headers: true,
            abbreviate_paths: true,
        }
    }

    /// Settings for medium terminals (16-20 rows)
    pub fn moderate() -> Self {
        Self {
            show_line_numbers: true,
            truncate_lines: true,
            max_line_length: 120,
            compact_headers: true,
            abbreviate_paths: true,
        }
    }

    /// Settings for normal terminals (> 20 rows)
    pub fn normal() -> Self {
        Self {
            show_line_numbers: true,
            truncate_lines: false,
            max_line_length: usize::MAX,
            compact_headers: false,
            abbreviate_paths: false,
        }
    }

    /// Get settings based on screen size
    pub fn for_screen_size(size: ScreenSize) -> Self {
        if size.rows < SHORT_TERMINAL_ROWS {
            Self::very_compact()
        } else if size.is_compact() {
            Self::moderate()
        } else {
            Self::normal()
        }
    }
}

/// Terminal capability detection
#[derive(Debug, Clone, Default)]
pub struct TerminalCapabilities {
    pub supports_unicode: bool,
    pub supports_truecolor: bool,
    pub supports_alternate_screen: bool,
    pub supports_insert_line: bool,
    pub supports_delete_line: bool,
    pub supports_mouse: bool,
    pub supports_focus_events: bool,
    pub supports_bracketed_paste: bool,
}

impl TerminalCapabilities {
    /// Detect capabilities from environment
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();

        let supports_unicode = true; // Most modern terminals support this
        let supports_truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        let supports_alternate_screen = true;
        let supports_insert_line = term.contains("xterm") || term.contains("screen");
        let supports_delete_line = term.contains("xterm") || term.contains("screen");
        let supports_mouse = Self::detect_mouse_support();
        let supports_focus_events = term.contains("xterm");
        let supports_bracketed_paste = term.contains("xterm") || term.contains("screen");

        Self {
            supports_unicode,
            supports_truecolor,
            supports_alternate_screen,
            supports_insert_line,
            supports_delete_line,
            supports_mouse,
            supports_focus_events,
            supports_bracketed_paste,
        }
    }

    fn detect_mouse_support() -> bool {
        let term = std::env::var("TERM").unwrap_or_default();

        // Check for common mouse-supporting terminals
        term.contains("xterm")
            || term.contains("kitty")
            || term.contains("wezterm")
            || term.contains("alacritty")
            || term.contains("screen")
            || term.contains("tmux")
    }

    /// Check if we can use ANSI escape codes for cursor positioning
    pub fn has_cursor_support(&self) -> bool {
        self.supports_alternate_screen || self.supports_insert_line
    }
}

/// Screen mode configuration
#[derive(Debug, Clone)]
pub struct ScreenModeConfig {
    pub mode: ScreenMode,
    pub capabilities: TerminalCapabilities,
    pub compact_settings: CompactSettings,
}

impl Default for ScreenModeConfig {
    fn default() -> Self {
        let capabilities = TerminalCapabilities::detect();
        let size = ScreenSize::from_terminal();

        // Determine mode
        let mode = if std::env::var("RUNIE_SCREEN_MODE").is_ok() {
            std::env::var("RUNIE_SCREEN_MODE")
                .ok()
                .and_then(|s| ScreenMode::parse(&s))
                .unwrap_or_default()
        } else {
            ScreenMode::detect()
        };

        let compact_settings = CompactSettings::for_screen_size(size);

        Self {
            mode,
            capabilities,
            compact_settings,
        }
    }
}

impl ScreenModeConfig {
    /// Create from explicit mode
    pub fn with_mode(mode: ScreenMode) -> Self {
        let capabilities = TerminalCapabilities::detect();
        let size = ScreenSize::from_terminal();
        let compact_settings = CompactSettings::for_screen_size(size);

        Self {
            mode,
            capabilities,
            compact_settings,
        }
    }

    /// Create forced fullscreen mode
    pub fn fullscreen() -> Self {
        Self::with_mode(ScreenMode::Fullscreen)
    }

    /// Create minimal mode
    pub fn minimal() -> Self {
        Self::with_mode(ScreenMode::Minimal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_mode_parse() {
        assert_eq!(ScreenMode::parse("fullscreen"), Some(ScreenMode::Fullscreen));
        assert_eq!(ScreenMode::parse("full"), Some(ScreenMode::Fullscreen));
        assert_eq!(ScreenMode::parse("inline"), Some(ScreenMode::Inline));
        assert_eq!(ScreenMode::parse("minimal"), Some(ScreenMode::Minimal));
        assert_eq!(ScreenMode::parse("unknown"), None);
    }

    #[test]
    fn test_screen_mode_display() {
        assert_eq!(ScreenMode::Fullscreen.to_string(), "fullscreen");
        assert_eq!(ScreenMode::Inline.to_string(), "inline");
        assert_eq!(ScreenMode::Minimal.to_string(), "minimal");
    }

    #[test]
    fn test_screen_size_defaults() {
        let size = ScreenSize::from_terminal();
        assert!(size.rows > 0);
        assert!(size.cols > 0);
    }

    #[test]
    fn test_compact_settings() {
        let small = ScreenSize { rows: 10, cols: 80 };
        let settings = CompactSettings::for_screen_size(small);
        assert!(!settings.show_line_numbers);

        let large = ScreenSize { rows: 40, cols: 120 };
        let settings = CompactSettings::for_screen_size(large);
        assert!(settings.show_line_numbers);
        assert!(!settings.truncate_lines);
    }

    #[test]
    fn test_terminal_capabilities() {
        let caps = TerminalCapabilities::detect();
        // At minimum, basic capabilities should be present
        assert!(caps.supports_unicode);
    }
}
