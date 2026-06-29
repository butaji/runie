//! Theme caching and quantization tests.

use std::sync::Arc;

use crate::terminal::caps::{MouseCapability, TermCaps};
use crate::theme::{BUILTIN_THEMES, current_theme, set_current_theme, set_current_theme_with_caps, test_lock};
use crate::theme::loader::{default_theme, minimal_fallback_theme};

#[test]
fn theme_cache_returns_same_instance() {
    let _lock = test_lock();
    set_current_theme("runie");
    let first = current_theme();
    set_current_theme("runie");
    let second = current_theme();
    assert!(Arc::ptr_eq(&first, &second));
}

fn truecolor_caps() -> TermCaps {
    TermCaps {
        truecolor: true,
        mouse: MouseCapability::Sgr,
        ..Default::default()
    }
}

fn ansi256_caps() -> TermCaps {
    TermCaps {
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
    for token in [
        "accent.primary",
        "success",
        "error",
        "text.primary",
        "accent.secondary",
    ] {
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
    for token in [
        "accent.primary",
        "success",
        "error",
        "text.primary",
        "accent.secondary",
    ] {
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

#[test]
fn builtin_theme_names_load_from_opaline() {
    let _lock = test_lock();
    for name in BUILTIN_THEMES {
        set_current_theme(name);
        let theme = current_theme();
        assert!(
            !theme.token_names().is_empty(),
            "theme {} should have tokens",
            name
        );
    }
    set_current_theme("runie");
}

#[test]
fn builtin_themes_have_distinct_bg_base() {
    let _lock = test_lock();
    set_current_theme_with_caps("runie", truecolor_caps());
    let runie_bg = current_theme().color("bg.base");

    set_current_theme_with_caps("dracula", truecolor_caps());
    let dracula_bg = current_theme().color("bg.base");
    assert_ne!(
        (runie_bg.r, runie_bg.g, runie_bg.b),
        (dracula_bg.r, dracula_bg.g, dracula_bg.b),
        "dracula bg.base should differ from runie"
    );

    set_current_theme_with_caps("nord", truecolor_caps());
    let nord_bg = current_theme().color("bg.base");
    assert_ne!(
        (dracula_bg.r, dracula_bg.g, dracula_bg.b),
        (nord_bg.r, nord_bg.g, nord_bg.b),
        "nord bg.base should differ from dracula"
    );

    set_current_theme_with_caps("catppuccin-latte", truecolor_caps());
    let latte_bg = current_theme().color("bg.base");
    assert!(
        latte_bg.r > 200 && latte_bg.g > 200 && latte_bg.b > 200,
        "catppuccin-latte bg.base should be light, got rgb({},{},{})",
        latte_bg.r,
        latte_bg.g,
        latte_bg.b
    );

    set_current_theme("runie");
}

// ── Layer 1 — State/Logic: theme fallback on invalid content ────────────────

/// Verifies that the embedded default theme loads without error.
/// This is a sanity check: if the build-pipeline corrupted DEFAULT_THEME_TOML,
/// this test would surface the regression.
#[test]
fn default_theme_loads_successfully() {
    let _lock = test_lock();
    let result = default_theme();
    assert!(
        result.is_ok(),
        "embedded default theme must load: {:?}",
        result.err()
    );
    let theme = result.unwrap();
    // The embedded theme must have at least basic tokens.
    assert!(!theme.token_names().is_empty(), "default theme should have tokens");
}

/// Verifies that the minimal fallback theme is always loadable.
/// This is the last-resort fallback used when ALL other loaders fail.
#[test]
fn minimal_fallback_theme_loads_successfully() {
    let _lock = test_lock();
    let theme = minimal_fallback_theme();
    assert!(!theme.token_names().is_empty(), "fallback theme should have tokens");
    // Verify the hardcoded color values are present.
    let bg = theme.color("bg-base");
    assert_ne!(bg, opaline::OpalineColor::FALLBACK, "fallback bg-base should resolve");
    let text = theme.color("text-primary");
    assert_ne!(text, opaline::OpalineColor::FALLBACK, "fallback text-primary should resolve");
}
