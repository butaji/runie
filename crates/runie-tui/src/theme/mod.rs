//! Theme system powered by opaline
//!
//! Runie-specific styles are registered as defaults so any theme can override them.
//! The current theme is cached in a global lock; `draw_snapshot` sets it at frame start.

pub use crate::semantic_tokens::SemanticTokens;

use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub(crate) mod colors;
pub(crate) mod glyph;
pub(crate) mod loader;
pub(crate) mod styles;

pub use colors::*;
pub use glyph::*;
pub use loader::{list_builtin_themes, BUILTIN_THEMES};
pub use styles::*;

#[cfg(test)]
mod tests;

static CURRENT_THEME: RwLock<Option<Arc<opaline::Theme>>> = RwLock::new(None);
static CURRENT_THEME_NAME: Mutex<String> = Mutex::new(String::new());
static CURRENT_CAPS: RwLock<Option<crate::terminal::caps::TermCaps>> = RwLock::new(None);

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub const DEFAULT_THEME_NAME: &str = "runie";

/// Set the active theme by name. Called by `draw_snapshot` at frame start.
/// This is a no-op when the requested theme is already active.
/// Uses the terminal capabilities last set by `set_current_theme_with_caps`,
/// falling back to default (no truecolor) caps if none were set.
pub fn set_current_theme(name: &str) {
    let caps = current_caps().unwrap_or_default();
    set_current_theme_with_caps(name, caps);
}

/// Set the active theme by name, quantized to the given terminal capabilities.
/// Quantization happens once at load time; per-frame rendering is unaffected.
pub fn set_current_theme_with_caps(name: &str, caps: crate::terminal::caps::TermCaps) {
    let name_same = CURRENT_THEME_NAME.lock().as_str() == name;
    let caps_same = CURRENT_CAPS.read().as_ref() == Some(&caps);
    if name_same && caps_same {
        return;
    }

    *CURRENT_CAPS.write() = Some(caps);
    *CURRENT_THEME_NAME.lock() = name.to_owned();
    let theme = loader::load_theme_with_caps(name, caps).unwrap_or_else(|_| loader::minimal_fallback_theme());
    *CURRENT_THEME.write() = Some(Arc::new(theme));
}

/// Get the name of the currently active theme.
pub fn current_theme_name() -> String {
    CURRENT_THEME_NAME.lock().clone()
}

fn current_caps() -> Option<crate::terminal::caps::TermCaps> {
    *CURRENT_CAPS.read()
}

/// Get the currently active theme (falls back to default).
pub fn current_theme() -> Arc<opaline::Theme> {
    let guard = CURRENT_THEME.read();
    guard.clone().unwrap_or_else(|| {
        // Drop the read lock before doing any load work.
        drop(guard);
        // No theme has been set yet (e.g. first frame, or tests that render
        // without calling `set_current_theme`). Load the embedded default and
        // register the runie semantic styles so `style_*()` accessors return
        // real colors (dim hints, borders, …) instead of empty/default styles.
        // This must match what `set_current_theme` → `load_theme` produces.
        // Only on a corrupted embedded theme do we fall back to the bare
        // minimal theme, intentionally left unregistered to avoid panicking on
        // its deliberately tiny token set.
        loader::default_theme()
            .map(|t| Arc::new(crate::theme::styles::register_runie_styles(t)))
            .unwrap_or_else(|_| Arc::new(loader::minimal_fallback_theme()))
    })
}

/// Get semantic tokens from the current theme.
pub fn semantic_tokens() -> SemanticTokens {
    SemanticTokens::from_theme(&current_theme())
}
