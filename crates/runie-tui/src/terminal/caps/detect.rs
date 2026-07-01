//! Private detection helpers for terminal capabilities.
//!
//! Relies on `supports-color` and `supports-hyperlinks` crates for capability detection.
//! No custom brand/multiplexer lookup tables are maintained.

use std::collections::HashMap;
use parking_lot::Mutex;

use super::ColorDepth;

/// Serializes environment access to avoid race conditions when tests run in parallel.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Wrapper that forces `supports-color` to run even when stdout is not a tty.
fn supports_color_forced() -> Option<supports_color::ColorLevel> {
    std::env::set_var("IGNORE_IS_TERMINAL", "1");
    let result = supports_color::on(supports_color::Stream::Stdout);
    std::env::remove_var("IGNORE_IS_TERMINAL");
    result
}

/// Detect color depth using `supports-color`.
pub(super) fn detect_color_depth(env: &HashMap<String, String>) -> ColorDepth {
    let _lock = ENV_MUTEX.lock();
    with_env(env, || {
        if let Some(level) = supports_color_forced() {
            if level.has_16m {
                return ColorDepth::Truecolor;
            } else if level.has_256 {
                return ColorDepth::ANSI256;
            } else if level.has_basic {
                return ColorDepth::ANSI16;
            }
        }
        // Conservative fallback: assume 256 colors
        ColorDepth::ANSI256
    })
}

/// Detect hyperlink support using `supports-hyperlinks`.
pub(super) fn detect_hyperlinks(env: &HashMap<String, String>) -> bool {
    let _lock = ENV_MUTEX.lock();
    with_env(env, supports_hyperlinks::supports_hyperlinks)
}

/// Temporarily set env vars from a snapshot, run a closure, then restore.
const ENV_KEYS: &[&str] = &[
    "COLORTERM",
    "TERM",
    "TERM_PROGRAM",
    "FORCE_COLOR",
    "FORCE_HYPERLINK",
    "NO_COLOR",
    "LC_ALL",
    "LC_CTYPE",
    "LANG",
    "IGNORE_IS_TERMINAL",
    // Multiplexer detection vars
    "TMUX",
    "ZELLIJ_SESSION_NAME",
    "STY",
    // Windows terminal
    "WT_SESSION",
];

fn with_env<R>(env: &HashMap<String, String>, f: impl FnOnce() -> R) -> R {
    let saved: Vec<_> = ENV_KEYS
        .iter()
        .map(|k| (k.to_string(), std::env::var(k).ok()))
        .collect();
    ENV_KEYS.iter().for_each(|k| std::env::remove_var(k));
    env.iter()
        .filter(|(k, _)| ENV_KEYS.contains(&k.as_str()))
        .for_each(|(k, v)| std::env::set_var(k, v));
    let result = f();
    saved.into_iter().for_each(|(k, old)| match old {
        Some(v) => std::env::set_var(&k, v),
        None => std::env::remove_var(&k),
    });
    result
}

/// Detect if we're running in a known modern terminal or multiplexer.
fn is_modern_terminal(env: &HashMap<String, String>) -> bool {
    // Check for known terminal programs
    if let Some(program) = env.get("TERM_PROGRAM") {
        match program.as_str() {
            "iTerm.app" | "vscode" | "WezTerm" | "Apple_Terminal" | "WarpTerminal"
            | "ghostty" => return true,
            _ => {}
        }
    }

    // Check for Windows Terminal
    if env.contains_key("WT_SESSION") {
        return true;
    }

    // Check for known terminal TERM values
    if let Some(term) = env.get("TERM") {
        match term.as_str() {
            "xterm-kitty" | "alacritty" => return true,
            _ => {}
        }
    }

    false
}

/// Detect if we're running in a terminal multiplexer.
fn is_in_multiplexer(env: &HashMap<String, String>) -> bool {
    env.contains_key("TMUX")
        || env.contains_key("ZELLIJ_SESSION_NAME")
        || env.contains_key("STY")
        || env
            .get("TERM")
            .map(|t| t.starts_with("screen"))
            .unwrap_or(false)
}

/// Detect mouse capability using environment information.
pub(super) fn detect_mouse(env: &HashMap<String, String>) -> super::MouseCapability {
    // If not in a known terminal or multiplexer, assume no mouse support
    let modern_terminal = is_modern_terminal(env);
    let in_multiplexer = is_in_multiplexer(env);

    if !modern_terminal && !in_multiplexer {
        return super::MouseCapability::None;
    }

    // In a multiplexer, use SGR protocol
    if in_multiplexer {
        return super::MouseCapability::Sgr;
    }

    // Modern terminals support SGR
    super::MouseCapability::Sgr
}

/// Detect clipboard support.
pub(super) fn detect_clipboard(env: &HashMap<String, String>) -> bool {
    // Modern terminals and tmux support clipboard operations
    if is_modern_terminal(env) {
        return true;
    }

    // tmux supports clipboard via OSC 52
    if env.contains_key("TMUX") {
        return true;
    }

    false
}

/// Detect focus tracking support.
pub(super) fn detect_focus_tracking(env: &HashMap<String, String>) -> bool {
    // Focus tracking is supported in modern terminals and multiplexers
    is_modern_terminal(env) || is_in_multiplexer(env)
}

/// Detect unicode support from locale environment.
pub(super) fn detect_unicode(env: &HashMap<String, String>) -> bool {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Some(value) = env.get(key) {
            let v = value.to_uppercase();
            if v.contains("UTF-8") || v.contains("UTF8") {
                return true;
            }
        }
    }
    // Conservative default: assume unicode support
    true
}
