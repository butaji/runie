//! Theme auto-detection based on terminal/system appearance.
//!
//! Detects whether to use a light or dark theme based on:
//! 1. `AppleInterfaceStyle` environment variable (macOS dark mode)
//! 2. `TERMCLICOLOR` environment variable (terminal color support hint)
//! 3. OSC 11 queries (dynamic terminal color detection - future)
//!
//! Grok uses the same approach for automatic theme switching.

use std::env;

/// Represents the detected system appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAppearance {
    Dark,
    Light,
}

impl SystemAppearance {
    /// Returns true if this is a dark appearance.
    pub fn is_dark(&self) -> bool {
        matches!(self, SystemAppearance::Dark)
    }

    /// Returns the default theme name for this appearance.
    pub fn default_theme_name(&self) -> &'static str {
        match self {
            SystemAppearance::Dark => "runie",
            SystemAppearance::Light => "catppuccin-latte",
        }
    }
}

/// Detect the system appearance by checking environment variables.
///
/// Priority:
/// 1. `AppleInterfaceStyle` (macOS) - set to "Dark" or "Light"
/// 2. `TERMCLICOLOR` - if set to "24bit" or "truecolor", likely supports dark
/// 3. Fallback to Dark (most common for terminal apps)
pub fn detect_system_appearance() -> SystemAppearance {
    // 1. Check AppleInterfaceStyle (macOS dark mode indicator)
    if let Ok(style) = env::var("AppleInterfaceStyle") {
        let style_lower = style.to_lowercase();
        if style_lower == "dark" {
            return SystemAppearance::Dark;
        } else if style_lower == "light" {
            return SystemAppearance::Light;
        }
    }

    // 2. Check TERMCLICOLOR (terminal color support)
    // This is a hint but not definitive about light/dark
    if let Ok(color) = env::var("TERMCLICOLOR") {
        let color_lower = color.to_lowercase();
        // If TERMCLICOLOR is set to a color, the terminal likely has a theme
        // Most modern terminals with color support default to dark for dev tools
        if !color_lower.is_empty() && color_lower != "no" && color_lower != "none" {
            // Terminal has color support - default to dark for dev tools
            return SystemAppearance::Dark;
        }
    }

    // 3. Fallback: check if we can detect from COLORFGBG (common in some terminals)
    // Format is usually "foreground;background" where background > 6 means light
    if let Ok(fgbg) = env::var("COLORFGBG") {
        let parts: Vec<&str> = fgbg.split(';').collect();
        if let Some(bg) = parts.get(1) {
            if let Ok(bg_num) = bg.parse::<u8>() {
                // Background color 0-6 is dark, 7-15 is light (extended ANSI)
                return if bg_num <= 6 {
                    SystemAppearance::Dark
                } else {
                    SystemAppearance::Light
                };
            }
        }
    }

    // Default to dark (most common for terminal/TUI apps)
    SystemAppearance::Dark
}

/// Get the appropriate theme name based on system appearance.
/// If a theme is already explicitly configured, returns None (no override).
pub fn get_theme_for_appearance(theme_override: Option<&str>) -> Option<String> {
    // Only auto-detect if no explicit theme is set
    if theme_override.is_some() {
        return None;
    }

    let appearance = detect_system_appearance();
    Some(appearance.default_theme_name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_appearance() {
        assert!(SystemAppearance::Dark.is_dark());
        assert!(!SystemAppearance::Light.is_dark());
    }

    #[test]
    fn test_default_theme_names() {
        assert_eq!(SystemAppearance::Dark.default_theme_name(), "runie");
        assert_eq!(
            SystemAppearance::Light.default_theme_name(),
            "catppuccin-latte"
        );
    }

    #[test]
    fn test_get_theme_for_appearance_with_override() {
        // When theme is explicitly set, don't override
        assert_eq!(get_theme_for_appearance(Some("nord")), None);
        assert_eq!(get_theme_for_appearance(Some("dracula")), None);
    }

    #[test]
    fn test_apple_interface_style_detection() {
        // Test with mocked environment
        let original = env::var("AppleInterfaceStyle").ok();

        env::set_var("AppleInterfaceStyle", "Dark");
        assert_eq!(detect_system_appearance(), SystemAppearance::Dark);

        env::set_var("AppleInterfaceStyle", "dark");
        assert_eq!(detect_system_appearance(), SystemAppearance::Dark);

        env::set_var("AppleInterfaceStyle", "Light");
        assert_eq!(detect_system_appearance(), SystemAppearance::Light);

        // Restore original
        match original {
            Some(v) => env::set_var("AppleInterfaceStyle", v),
            None => env::remove_var("AppleInterfaceStyle"),
        }
    }

    #[test]
    fn test_term_color_detection() {
        let original = env::var("TERMCLICOLOR").ok();

        env::set_var("TERMCLICOLOR", "truecolor");
        assert_eq!(detect_system_appearance(), SystemAppearance::Dark);

        // Restore original
        match original {
            Some(v) => env::set_var("TERMCLICOLOR", v),
            None => env::remove_var("TERMCLICOLOR"),
        }
    }
}
