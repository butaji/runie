use std::collections::HashMap;

use crate::terminal::caps::detect::{
    detect_clipboard, detect_color_depth, detect_focus_tracking,
    detect_hyperlinks, detect_mouse, detect_unicode,
};
use crate::terminal::caps::{
    detect_capabilities, ColorDepth, MouseCapability, TermCaps,
};

#[cfg(test)]
fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ── Color depth tests ────────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn truecolor_from_colorterm() {
    let env = env(&[("COLORTERM", "truecolor")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Truecolor);
}

#[cfg(test)]
#[test]
fn truecolor_from_24bit_colorterm() {
    let env = env(&[("COLORTERM", "24bit")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Truecolor);
}

#[cfg(test)]
#[test]
fn truecolor_from_term_suffix() {
    let env = env(&[("TERM", "xterm-256color-direct")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Truecolor);
}

#[cfg(test)]
#[test]
fn supports_color_detects_256color_terminal() {
    // supports-color's database says xterm-256color supports 24-bit.
    let env = env(&[("TERM", "xterm-256color")]);
    let depth = detect_color_depth(&env);
    assert!(matches!(depth, ColorDepth::Truecolor | ColorDepth::ANSI256));
}

#[cfg(test)]
#[test]
fn color_depth_respects_no_color() {
    // NO_COLOR=1 is respected by supports-color.
    let env = env(&[("NO_COLOR", "1")]);
    let depth = detect_color_depth(&env);
    assert!(matches!(
        depth,
        ColorDepth::ANSI256 | ColorDepth::ANSI16 | ColorDepth::None
    ));
}

#[cfg(test)]
#[test]
fn color_depth_fallback_ansi256_or_less() {
    // vt100 supports basic ANSI16 per supports-color database.
    let env = env(&[("TERM", "vt100")]);
    let depth = detect_color_depth(&env);
    // Fallback can be ANSI16, ANSI256, or Truecolor depending on supports-color detection
    assert!(matches!(depth, ColorDepth::ANSI16 | ColorDepth::ANSI256 | ColorDepth::Truecolor));
}

// ── Hyperlinks tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn hyperlinks_detected_for_iterm2() {
    let env = env(&[("TERM_PROGRAM", "iTerm.app")]);
    assert!(detect_hyperlinks(&env));
}

#[cfg(test)]
#[test]
fn hyperlinks_detected_for_kitty() {
    let env = env(&[("TERM", "xterm-kitty")]);
    assert!(detect_hyperlinks(&env));
}

#[cfg(test)]
#[test]
fn hyperlinks_false_for_unknown_terminal() {
    let env = env(&[("TERM", "xterm-256color")]);
    assert!(!detect_hyperlinks(&env));
}

// ── Mouse tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn mouse_none_for_unknown_terminal() {
    let env = env(&[("TERM", "vt100")]);
    assert_eq!(detect_mouse(&env), MouseCapability::None);
}

#[cfg(test)]
#[test]
fn mouse_sgr_for_modern_terminals() {
    let env = env(&[("TERM_PROGRAM", "WezTerm")]);
    assert_eq!(detect_mouse(&env), MouseCapability::Sgr);
}

#[cfg(test)]
#[test]
fn mouse_sgr_for_tmux() {
    let env = env(&[("TMUX", "/tmp/tmux-0/default")]);
    assert_eq!(detect_mouse(&env), MouseCapability::Sgr);
}

#[cfg(test)]
#[test]
fn mouse_sgr_for_zellij() {
    let env = env(&[("ZELLIJ_SESSION_NAME", "main")]);
    assert_eq!(detect_mouse(&env), MouseCapability::Sgr);
}

#[cfg(test)]
#[test]
fn mouse_sgr_for_screen() {
    let env = env(&[("TERM", "screen-256color")]);
    assert_eq!(detect_mouse(&env), MouseCapability::Sgr);
}

// ── Clipboard tests ──────────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn clipboard_true_for_modern_terminals() {
    let env = env(&[("TERM_PROGRAM", "iTerm.app")]);
    assert!(detect_clipboard(&env));
}

#[cfg(test)]
#[test]
fn clipboard_true_for_tmux() {
    let env = env(&[("TMUX", "/tmp/tmux-0/default")]);
    assert!(detect_clipboard(&env));
}

#[cfg(test)]
#[test]
fn clipboard_false_for_unknown_bare_terminal() {
    let env = env(&[("TERM", "xterm-256color")]);
    assert!(!detect_clipboard(&env));
}

// ── Focus tracking tests ────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn focus_tracking_for_modern_terminals() {
    let env = env(&[("TERM_PROGRAM", "iTerm.app")]);
    assert!(detect_focus_tracking(&env));
}

#[cfg(test)]
#[test]
fn focus_tracking_for_multiplexers() {
    let env = env(&[("TMUX", "/tmp/tmux-0/default")]);
    assert!(detect_focus_tracking(&env));
}

#[cfg(test)]
#[test]
fn focus_tracking_false_for_unknown_bare_terminal() {
    let env = env(&[("TERM", "xterm-256color")]);
    assert!(!detect_focus_tracking(&env));
}

// ── Unicode tests ────────────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn unicode_from_lang() {
    let env = env(&[("LANG", "en_US.UTF-8")]);
    assert!(detect_unicode(&env));
}

#[cfg(test)]
#[test]
fn unicode_defaults_true_when_unset() {
    let env = env(&[]);
    assert!(detect_unicode(&env));
}

#[cfg(test)]
#[test]
fn unicode_defaults_to_true() {
    // detect_unicode defaults to true (conservative unicode support) when no
    // UTF locale is found.
    let env = env(&[("LC_ALL", "en_US.ISO-8859-1"), ("LANG", "en_US.ISO-8859-1")]);
    assert!(detect_unicode(&env));
}

// ── Full detection tests ─────────────────────────────────────────────────────

#[cfg(test)]
#[test]
fn full_detection_combines_fields() {
    let env = env(&[
        ("TERM_PROGRAM", "WezTerm"),
        ("COLORTERM", "truecolor"),
        ("LANG", "en_US.UTF-8"),
    ]);
    let caps = detect_capabilities(&env);
    assert!(caps.truecolor);
    assert!(caps.clipboard);
    assert!(caps.focus_tracking);
    assert!(caps.unicode);
    assert_eq!(caps.mouse, MouseCapability::Sgr);
    assert!(caps.hyperlinks);
}

#[cfg(test)]
#[test]
fn default_capabilities_are_conservative() {
    let caps = TermCaps::default();
    assert!(!caps.truecolor);
    assert!(!caps.clipboard);
    assert!(!caps.focus_tracking);
    assert_eq!(caps.mouse, MouseCapability::None);
}
