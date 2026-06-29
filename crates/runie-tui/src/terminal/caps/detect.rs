//! Private detection helpers for terminal capabilities.

use std::collections::HashMap;
use std::sync::Mutex;

use super::{ColorDepth, MouseCapability, MultiplexerType};

/// Serializes environment access to avoid race conditions when tests run in parallel.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Wrapper that forces `supports-color` to run even when stdout is not a tty.
fn supports_color_forced() -> Option<supports_color::ColorLevel> {
    std::env::set_var("IGNORE_IS_TERMINAL", "1");
    let result = supports_color::on(supports_color::Stream::Stdout);
    std::env::remove_var("IGNORE_IS_TERMINAL");
    result
}

/// Detect color depth using `supports-color`, augmented with brand/multiplexer fallback.
pub(super) fn detect_color_depth(
    env: &HashMap<String, String>,
    brand: super::TerminalBrand,
    multiplexer: super::MultiplexerType,
) -> ColorDepth {
    // Known modern terminals always support truecolor, regardless of TERM database.
    let brand_truecolor = matches!(
        brand,
        super::TerminalBrand::ITerm2
            | super::TerminalBrand::WezTerm
            | super::TerminalBrand::Alacritty
            | super::TerminalBrand::Kitty
            | super::TerminalBrand::WindowsTerminal
            | super::TerminalBrand::Warp
            | super::TerminalBrand::Ghostty,
    ) || multiplexer == MultiplexerType::Tmux;

    if brand_truecolor {
        return ColorDepth::Truecolor;
    }

    let _lock = ENV_MUTEX.lock().unwrap();
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
        ColorDepth::ANSI256
    })
}

/// Detect hyperlink support using `supports-hyperlinks`.
pub(super) fn detect_hyperlinks(env: &HashMap<String, String>) -> bool {
    let _lock = ENV_MUTEX.lock().unwrap();
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

pub(super) fn detect_brand(env: &HashMap<String, String>) -> super::TerminalBrand {
    if let Some(program) = env.get("TERM_PROGRAM") {
        match program.as_str() {
            "iTerm.app" => return super::TerminalBrand::ITerm2,
            "vscode" => return super::TerminalBrand::VSCode,
            "WezTerm" => return super::TerminalBrand::WezTerm,
            "Apple_Terminal" => return super::TerminalBrand::TerminalApp,
            "WarpTerminal" => return super::TerminalBrand::Warp,
            "ghostty" => return super::TerminalBrand::Ghostty,
            _ => {}
        }
    }

    if env.contains_key("WT_SESSION") {
        return super::TerminalBrand::WindowsTerminal;
    }

    if let Some(term) = env.get("TERM") {
        match term.as_str() {
            "xterm-kitty" => return super::TerminalBrand::Kitty,
            "alacritty" => return super::TerminalBrand::Alacritty,
            _ => {}
        }
    }

    super::TerminalBrand::Unknown
}

pub(super) fn detect_multiplexer(env: &HashMap<String, String>) -> MultiplexerType {
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

pub(super) fn detect_mouse(
    brand: super::TerminalBrand,
    multiplexer: MultiplexerType,
) -> MouseCapability {
    if brand == super::TerminalBrand::Unknown && multiplexer == MultiplexerType::None {
        return MouseCapability::None;
    }
    if multiplexer != MultiplexerType::None {
        return MouseCapability::Sgr;
    }
    match brand {
        super::TerminalBrand::Kitty | super::TerminalBrand::Ghostty => MouseCapability::SgrExtended,
        super::TerminalBrand::ITerm2
        | super::TerminalBrand::WezTerm
        | super::TerminalBrand::Alacritty
        | super::TerminalBrand::WindowsTerminal
        | super::TerminalBrand::Warp
        | super::TerminalBrand::VSCode => MouseCapability::Sgr,
        super::TerminalBrand::TerminalApp | super::TerminalBrand::Unknown => {
            MouseCapability::Legacy
        }
    }
}

pub(super) fn detect_clipboard(brand: super::TerminalBrand, multiplexer: MultiplexerType) -> bool {
    matches!(
        brand,
        super::TerminalBrand::ITerm2
            | super::TerminalBrand::WezTerm
            | super::TerminalBrand::Kitty
            | super::TerminalBrand::Alacritty
            | super::TerminalBrand::WindowsTerminal
            | super::TerminalBrand::Warp
            | super::TerminalBrand::Ghostty
            | super::TerminalBrand::VSCode
    ) || multiplexer == MultiplexerType::Tmux
}

pub(super) fn detect_focus_tracking(
    brand: super::TerminalBrand,
    multiplexer: MultiplexerType,
) -> bool {
    brand != super::TerminalBrand::Unknown || multiplexer != MultiplexerType::None
}

pub(super) fn detect_unicode(env: &HashMap<String, String>) -> bool {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Some(value) = env.get(key) {
            let v = value.to_uppercase();
            if v.contains("UTF-8") || v.contains("UTF8") {
                return true;
            }
        }
    }
    true
}
