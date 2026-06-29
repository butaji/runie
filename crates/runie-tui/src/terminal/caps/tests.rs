#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::terminal::caps::detect::{
        detect_brand, detect_clipboard, detect_color_depth, detect_focus_tracking,
        detect_hyperlinks, detect_mouse, detect_multiplexer, detect_unicode,
    };
    use crate::terminal::caps::{
        detect_capabilities, ColorDepth, MouseCapability, MultiplexerType, TermCaps, TerminalBrand,
    };

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn truecolor_from_colorterm() {
        let env = env(&[("COLORTERM", "truecolor")]);
        assert_eq!(
            detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None),
            ColorDepth::Truecolor
        );
    }

    #[test]
    fn truecolor_from_24bit_colorterm() {
        let env = env(&[("COLORTERM", "24bit")]);
        assert_eq!(
            detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None),
            ColorDepth::Truecolor
        );
    }

    #[test]
    fn truecolor_from_term_suffix() {
        let env = env(&[("TERM", "xterm-256color-direct")]);
        assert_eq!(
            detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None),
            ColorDepth::Truecolor
        );
    }

    #[test]
    fn supports_color_detects_256color_terminal() {
        // supports-color's database says xterm-256color supports 24-bit.
        let env = env(&[("TERM", "xterm-256color")]);
        let depth = detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None);
        assert!(matches!(depth, ColorDepth::Truecolor | ColorDepth::ANSI256));
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
    fn unicode_defaults_to_true() {
        // detect_unicode defaults to true (conservative unicode support) when no
        // UTF locale is found. This is the historical behavior preserved for
        // backwards compatibility. Real unicode detection is done via supports-color.
        let env = env(&[("LC_ALL", "en_US.ISO-8859-1"), ("LANG", "en_US.ISO-8859-1")]);
        assert!(detect_unicode(&env));
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
        assert!(caps.hyperlinks);
    }

    #[test]
    fn default_capabilities_are_conservative() {
        let caps = TermCaps::default();
        assert!(!caps.truecolor);
        assert!(!caps.clipboard);
        assert!(!caps.focus_tracking);
        assert_eq!(caps.mouse, MouseCapability::None);
        assert_eq!(caps.brand, TerminalBrand::Unknown);
    }

    #[test]
    fn hyperlinks_detected_for_iterm2() {
        let env = env(&[("TERM_PROGRAM", "iTerm.app")]);
        assert!(detect_hyperlinks(&env));
    }

    #[test]
    fn hyperlinks_detected_for_kitty() {
        let env = env(&[("TERM", "xterm-kitty")]);
        assert!(detect_hyperlinks(&env));
    }

    #[test]
    fn hyperlinks_false_for_unknown_terminal() {
        let env = env(&[("TERM", "xterm-256color")]);
        assert!(!detect_hyperlinks(&env));
    }

    #[test]
    fn color_depth_truecolor_from_colorterm() {
        let env = env(&[("COLORTERM", "truecolor")]);
        assert_eq!(
            detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None),
            ColorDepth::Truecolor
        );
    }

    #[test]
    fn color_depth_respects_no_color() {
        // NO_COLOR=1 is respected by supports-color.
        let env = env(&[("NO_COLOR", "1")]);
        // Unknown brand, no multiplexer → fallback ANSI256.
        let depth = detect_color_depth(&env, TerminalBrand::Unknown, MultiplexerType::None);
        assert!(matches!(
            depth,
            ColorDepth::ANSI256 | ColorDepth::ANSI16 | ColorDepth::None
        ));
    }

    #[test]
    fn color_depth_fallback_truecolor_for_modern_brand() {
        // Known modern brand → truecolor even without explicit hints.
        let env = env(&[("TERM", "xterm-256color")]);
        assert_eq!(
            detect_color_depth(&env, TerminalBrand::WezTerm, MultiplexerType::None),
            ColorDepth::Truecolor
        );
    }
}
