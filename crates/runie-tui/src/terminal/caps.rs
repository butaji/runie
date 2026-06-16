//! Terminal capability detection.
//!
//! Heuristic detection of the terminal emulator / multiplexer and the
//! features it is likely to support (truecolor, mouse, clipboard, focus
//! tracking, unicode). Detection is purely functional over an environment
//! snapshot so it can be unit-tested without touching `std::env`.

use std::collections::HashMap;

/// Detected terminal capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    pub truecolor: bool,
    pub mouse: MouseCapability,
    pub clipboard: bool,
    pub focus_tracking: bool,
    pub unicode: bool,
    pub brand: TerminalBrand,
    pub multiplexer: MultiplexerType,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self {
            truecolor: false,
            mouse: MouseCapability::None,
            clipboard: false,
            focus_tracking: false,
            unicode: true,
            brand: TerminalBrand::Unknown,
            multiplexer: MultiplexerType::None,
        }
    }
}

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
pub fn detect_capabilities_from_env() -> TerminalCapabilities {
    let env: HashMap<String, String> = std::env::vars().collect();
    detect_capabilities(&env)
}

/// Detect capabilities from an explicit environment snapshot.
pub fn detect_capabilities(env: &HashMap<String, String>) -> TerminalCapabilities {
    let brand = detect_brand(env);
    let multiplexer = detect_multiplexer(env);
    let truecolor = detect_truecolor(env, brand, multiplexer);

    TerminalCapabilities {
        truecolor,
        mouse: detect_mouse(brand, multiplexer),
        clipboard: detect_clipboard(brand, multiplexer),
        focus_tracking: detect_focus_tracking(brand, multiplexer),
        unicode: detect_unicode(env),
        brand,
        multiplexer,
    }
}

fn detect_truecolor(
    env: &HashMap<String, String>,
    brand: TerminalBrand,
    multiplexer: MultiplexerType,
) -> bool {
    if let Some(ct) = env.get("COLORTERM") {
        let ct = ct.to_lowercase();
        if ct == "truecolor" || ct == "24bit" {
            return true;
        }
    }

    if let Some(term) = env.get("TERM") {
        let term = term.to_lowercase();
        if term.ends_with("-direct") || term.ends_with("-truecolor") || term.contains("truecolor") {
            return true;
        }
    }

    // Modern terminals that unconditionally support 24-bit color.
    matches!(
        brand,
        TerminalBrand::ITerm2
            | TerminalBrand::WezTerm
            | TerminalBrand::Alacritty
            | TerminalBrand::Kitty
            | TerminalBrand::WindowsTerminal
            | TerminalBrand::Warp
            | TerminalBrand::Ghostty
    ) || multiplexer == MultiplexerType::Tmux
}

fn detect_brand(env: &HashMap<String, String>) -> TerminalBrand {
    if let Some(program) = env.get("TERM_PROGRAM") {
        match program.as_str() {
            "iTerm.app" => return TerminalBrand::ITerm2,
            "vscode" => return TerminalBrand::VSCode,
            "WezTerm" => return TerminalBrand::WezTerm,
            "Apple_Terminal" => return TerminalBrand::TerminalApp,
            "WarpTerminal" => return TerminalBrand::Warp,
            "ghostty" => return TerminalBrand::Ghostty,
            _ => {}
        }
    }

    if env.contains_key("WT_SESSION") {
        return TerminalBrand::WindowsTerminal;
    }

    if let Some(term) = env.get("TERM") {
        match term.as_str() {
            "xterm-kitty" => return TerminalBrand::Kitty,
            "alacritty" => return TerminalBrand::Alacritty,
            _ => {}
        }
    }

    TerminalBrand::Unknown
}

fn detect_multiplexer(env: &HashMap<String, String>) -> MultiplexerType {
    if env.contains_key("TMUX") {
        return MultiplexerType::Tmux;
    }

    if env.contains_key("ZELLIJ_SESSION_NAME") {
        return MultiplexerType::Zellij;
    }

    if env.contains_key("STY") {
        return MultiplexerType::Screen;
    }

    if let Some(term) = env.get("TERM") {
        if term.starts_with("screen") {
            return MultiplexerType::Screen;
        }
    }

    MultiplexerType::None
}

fn detect_mouse(brand: TerminalBrand, multiplexer: MultiplexerType) -> MouseCapability {
    if brand == TerminalBrand::Unknown && multiplexer == MultiplexerType::None {
        return MouseCapability::None;
    }

    if multiplexer != MultiplexerType::None {
        // Modern multiplexers all understand SGR coordinates.
        return MouseCapability::Sgr;
    }

    match brand {
        TerminalBrand::Kitty | TerminalBrand::Ghostty => MouseCapability::SgrExtended,
        TerminalBrand::ITerm2
        | TerminalBrand::WezTerm
        | TerminalBrand::Alacritty
        | TerminalBrand::WindowsTerminal
        | TerminalBrand::Warp
        | TerminalBrand::VSCode => MouseCapability::Sgr,
        TerminalBrand::TerminalApp | TerminalBrand::Unknown => MouseCapability::Legacy,
    }
}

fn detect_clipboard(brand: TerminalBrand, multiplexer: MultiplexerType) -> bool {
    // OSC 52 works reliably in these emulators and in tmux with passthrough.
    matches!(
        brand,
        TerminalBrand::ITerm2
            | TerminalBrand::WezTerm
            | TerminalBrand::Kitty
            | TerminalBrand::Alacritty
            | TerminalBrand::WindowsTerminal
            | TerminalBrand::Warp
            | TerminalBrand::Ghostty
            | TerminalBrand::VSCode
    ) || multiplexer == MultiplexerType::Tmux
}

fn detect_focus_tracking(brand: TerminalBrand, multiplexer: MultiplexerType) -> bool {
    // Focus tracking (CSI ? 1004) is widely supported by modern terminals
    // and multiplexers.
    brand != TerminalBrand::Unknown || multiplexer != MultiplexerType::None
}

fn detect_unicode(env: &HashMap<String, String>) -> bool {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Some(value) = env.get(key) {
            return value.to_uppercase().contains("UTF-8") || value.to_uppercase().contains("UTF8");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn truecolor_from_colorterm() {
        let env = env(&[("COLORTERM", "truecolor")]);
        assert!(detect_truecolor(
            &env,
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn truecolor_from_24bit_colorterm() {
        let env = env(&[("COLORTERM", "24bit")]);
        assert!(detect_truecolor(
            &env,
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn truecolor_from_term_suffix() {
        let env = env(&[("TERM", "xterm-256color-direct")]);
        assert!(detect_truecolor(
            &env,
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn no_truecolor_without_hints() {
        let env = env(&[("TERM", "xterm-256color")]);
        assert!(!detect_truecolor(
            &env,
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn brand_iterm2() {
        let env = env(&[("TERM_PROGRAM", "iTerm.app")]);
        assert_eq!(detect_brand(&env), TerminalBrand::ITerm2);
    }

    #[test]
    fn brand_vscode() {
        let env = env(&[("TERM_PROGRAM", "vscode")]);
        assert_eq!(detect_brand(&env), TerminalBrand::VSCode);
    }

    #[test]
    fn brand_windows_terminal() {
        let env = env(&[("WT_SESSION", "{00000000-0000-0000-0000-000000000000}")]);
        assert_eq!(detect_brand(&env), TerminalBrand::WindowsTerminal);
    }

    #[test]
    fn brand_kitty_from_term() {
        let env = env(&[("TERM", "xterm-kitty")]);
        assert_eq!(detect_brand(&env), TerminalBrand::Kitty);
    }

    #[test]
    fn brand_unknown_when_empty() {
        let env = env(&[]);
        assert_eq!(detect_brand(&env), TerminalBrand::Unknown);
    }

    #[test]
    fn multiplexer_tmux() {
        let env = env(&[("TMUX", "/tmp/tmux-0/default")]);
        assert_eq!(detect_multiplexer(&env), MultiplexerType::Tmux);
    }

    #[test]
    fn multiplexer_zellij() {
        let env = env(&[("ZELLIJ_SESSION_NAME", "main")]);
        assert_eq!(detect_multiplexer(&env), MultiplexerType::Zellij);
    }

    #[test]
    fn multiplexer_screen_from_term() {
        let env = env(&[("TERM", "screen-256color")]);
        assert_eq!(detect_multiplexer(&env), MultiplexerType::Screen);
    }

    #[test]
    fn multiplexer_none() {
        let env = env(&[("TERM", "xterm-256color")]);
        assert_eq!(detect_multiplexer(&env), MultiplexerType::None);
    }

    #[test]
    fn mouse_none_for_unknown_terminal() {
        assert_eq!(
            detect_mouse(TerminalBrand::Unknown, MultiplexerType::None),
            MouseCapability::None
        );
    }

    #[test]
    fn mouse_sgr_for_modern_terminals() {
        assert_eq!(
            detect_mouse(TerminalBrand::WezTerm, MultiplexerType::None),
            MouseCapability::Sgr
        );
        assert_eq!(
            detect_mouse(TerminalBrand::Alacritty, MultiplexerType::None),
            MouseCapability::Sgr
        );
    }

    #[test]
    fn mouse_sgr_for_multiplexers() {
        assert_eq!(
            detect_mouse(TerminalBrand::Unknown, MultiplexerType::Tmux),
            MouseCapability::Sgr
        );
    }

    #[test]
    fn clipboard_true_for_supported_terminals() {
        assert!(detect_clipboard(
            TerminalBrand::ITerm2,
            MultiplexerType::None
        ));
        assert!(detect_clipboard(
            TerminalBrand::Unknown,
            MultiplexerType::Tmux
        ));
    }

    #[test]
    fn clipboard_false_for_unknown_terminal() {
        assert!(!detect_clipboard(
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn focus_tracking_for_modern_or_multiplexer() {
        assert!(detect_focus_tracking(
            TerminalBrand::ITerm2,
            MultiplexerType::None
        ));
        assert!(detect_focus_tracking(
            TerminalBrand::Unknown,
            MultiplexerType::Tmux
        ));
    }

    #[test]
    fn focus_tracking_false_for_unknown_bare_terminal() {
        assert!(!detect_focus_tracking(
            TerminalBrand::Unknown,
            MultiplexerType::None
        ));
    }

    #[test]
    fn unicode_from_lang() {
        let env = env(&[("LANG", "en_US.UTF-8")]);
        assert!(detect_unicode(&env));
    }

    #[test]
    fn unicode_defaults_true_when_unset() {
        let env = env(&[]);
        assert!(detect_unicode(&env));
    }

    #[test]
    fn non_unicode_locale() {
        let env = env(&[("LANG", "en_US.ISO-8859-1")]);
        assert!(!detect_unicode(&env));
    }

    #[test]
    fn full_detection_combines_fields() {
        let env = env(&[
            ("TERM_PROGRAM", "WezTerm"),
            ("COLORTERM", "truecolor"),
            ("LANG", "en_US.UTF-8"),
        ]);
        let caps = detect_capabilities(&env);
        assert_eq!(caps.brand, TerminalBrand::WezTerm);
        assert!(caps.truecolor);
        assert!(caps.clipboard);
        assert!(caps.focus_tracking);
        assert!(caps.unicode);
        assert_eq!(caps.mouse, MouseCapability::Sgr);
        assert_eq!(caps.multiplexer, MultiplexerType::None);
    }

    #[test]
    fn default_capabilities_are_conservative() {
        let caps = TerminalCapabilities::default();
        assert!(!caps.truecolor);
        assert!(!caps.clipboard);
        assert!(!caps.focus_tracking);
        assert_eq!(caps.mouse, MouseCapability::None);
        assert_eq!(caps.brand, TerminalBrand::Unknown);
    }
}
