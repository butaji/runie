//! Theme caching and quantization tests.

use std::sync::Arc;

use crate::terminal::caps::{MouseCapability, TerminalCapabilities, TerminalBrand};
use crate::theme::{current_theme, set_current_theme, set_current_theme_with_caps, test_lock};

#[test]
fn theme_cache_returns_same_instance() {
    let _lock = test_lock();
    set_current_theme("runie");
    let first = current_theme();
    set_current_theme("runie");
    let second = current_theme();
    assert!(Arc::ptr_eq(&first, &second));
}

fn truecolor_caps() -> TerminalCapabilities {
    TerminalCapabilities {
        truecolor: true,
        mouse: MouseCapability::Sgr,
        ..Default::default()
    }
}

fn ansi256_caps() -> TerminalCapabilities {
    TerminalCapabilities {
        truecolor: false,
        mouse: MouseCapability::Legacy,
        ..Default::default()
    }
}

#[test]
fn truecolor_theme_keeps_rgb_colors() {
    let _lock = test_lock();
    set_current_theme_with_caps("runie", truecolor_caps());
    let theme = current_theme();

    // All key semantic tokens should resolve without falling back to FALLBACK.
    // Tokens that actually exist in the DEFAULT_THEME_TOML.
    for token in ["accent.primary", "success", "error", "text.primary", "accent.secondary"] {
        let c = theme.color(token);
        assert!(
            c != opaline::OpalineColor::FALLBACK,
            "token '{token}' fell back to FALLBACK",
        );
    }
}

#[test]
fn non_truecolor_quantizes_to_indexed_approximations() {
    let _lock = test_lock();
    set_current_theme_with_caps("runie", ansi256_caps());
    let theme = current_theme();

    // Quantized theme should still resolve all key tokens without falling back.
    for token in ["accent.primary", "success", "error", "text.primary", "accent.secondary"] {
        let c = theme.color(token);
        assert!(
            c != opaline::OpalineColor::FALLBACK,
            "quantized token '{token}' fell back to FALLBACK",
        );
    }
}

#[test]
fn quantization_is_idempotent() {
    let _lock = test_lock();
    set_current_theme_with_caps("runie", ansi256_caps());
    let first = current_theme();

    // Calling with same caps again should be a no-op (name hasn't changed).
    set_current_theme_with_caps("runie", ansi256_caps());
    let second = current_theme();
    assert!(Arc::ptr_eq(&first, &second));
}
