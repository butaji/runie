//! Theme caching, quantization, and glyph tests.

use std::sync::Arc;

use crate::terminal::caps::{MouseCapability, TermCaps};
use crate::theme::glyph::{
    BOX_BOTTOM_LEFT, BOX_BOTTOM_RIGHT, BOX_HORIZONTAL, BOX_TOP_LEFT, BOX_TOP_RIGHT, BOX_VERTICAL,
    GLYPH_BULLET, GLYPH_CHECK, GLYPH_CHECKED, GLYPH_DOWNLOAD, GLYPH_FILTER, GLYPH_SELECTED,
    GLYPH_SPINNER, GLYPH_TOOL, GLYPH_UNCHECKED, GLYPH_UNSELECTED, GLYPH_X, INDICATOR_COLLAPSED,
    INDICATOR_ERROR, PANEL_CHAT, PANEL_INPUT, SCROLLBAR_THUMB, SCROLLBAR_TRACK,
};
use crate::theme::loader::{default_theme, minimal_fallback_theme};
use crate::theme::{
    current_theme, set_current_theme, set_current_theme_with_caps, test_lock, BUILTIN_THEMES,
};

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
    assert!(
        !theme.token_names().is_empty(),
        "default theme should have tokens"
    );
}

/// Verifies that the minimal fallback theme is always loadable.
/// This is the last-resort fallback used when ALL other loaders fail.
#[test]
fn minimal_fallback_theme_loads_successfully() {
    let _lock = test_lock();
    let theme = minimal_fallback_theme();
    assert!(
        !theme.token_names().is_empty(),
        "fallback theme should have tokens"
    );
    // Verify the hardcoded color values are present.
    let bg = theme.color("bg-base");
    assert_ne!(
        bg,
        opaline::OpalineColor::FALLBACK,
        "fallback bg-base should resolve"
    );
    let text = theme.color("text-primary");
    assert_ne!(
        text,
        opaline::OpalineColor::FALLBACK,
        "fallback text-primary should resolve"
    );
}

// ── Layer 1 — State/Logic: glyph constants ────────────────────────────────────

/// Verifies that all checkbox glyphs have correct values.
#[test]
fn glyph_checkbox_constants_are_correct() {
    assert_eq!(GLYPH_CHECKED, "[x]");
    assert_eq!(GLYPH_UNCHECKED, "[ ]");
    assert_eq!(GLYPH_CHECK, "✓");
    assert_eq!(GLYPH_X, "✗");
}

/// Verifies that all selection and navigation glyphs have correct values.
#[test]
fn glyph_selection_constants_are_correct() {
    assert_eq!(GLYPH_SELECTED, "▸ ");
    assert_eq!(GLYPH_UNSELECTED, "  ");
}

/// Verifies that tool and status glyphs have correct values.
#[test]
fn glyph_tool_constants_are_correct() {
    assert_eq!(GLYPH_TOOL, "◆ ");
    assert_eq!(GLYPH_BULLET, "•");
}

/// Verifies that indicator glyphs have correct values.
#[test]
fn glyph_indicator_constants_are_correct() {
    assert_eq!(INDICATOR_COLLAPSED, " [+]");
    assert_eq!(INDICATOR_ERROR, " [✗]");
}

/// Verifies that scrollbar glyphs have correct values.
#[test]
fn glyph_scrollbar_constants_are_correct() {
    assert_eq!(SCROLLBAR_TRACK, " ");
    assert_eq!(SCROLLBAR_THUMB, "▐");
}

/// Verifies that panel header glyphs have correct values.
#[test]
fn glyph_panel_constants_are_correct() {
    assert_eq!(PANEL_CHAT, " Chat ");
    assert_eq!(PANEL_INPUT, " Input ");
}

/// Verifies that spinner glyph is a braille character.
#[test]
fn glyph_spinner_is_braille() {
    // GLYPH_SPINNER is a char (first frame of braille spinner)
    // Verify it's in the braille range (U+2800 to U+28FF)
    let c = GLYPH_SPINNER;
    assert!(
        ('\u{2800}'..='\u{28FF}').contains(&c),
        "GLYPH_SPINNER should be a braille character, got: {c}"
    );
}

/// Verifies that filter glyph is the correct character.
#[test]
fn glyph_filter_is_correct() {
    assert_eq!(GLYPH_FILTER, '❯');
}

/// Verifies that download glyph is correct.
#[test]
fn glyph_download_is_correct() {
    assert_eq!(GLYPH_DOWNLOAD, "⇣");
}

/// The thinking/waiting line must use an animated braille spinner (not the
/// static ◐), the grok wording with a single `…` glyph, and grok's timer
/// format: one decimal below 10s, integer at ≥10s. See GROK.md §24.
#[test]
fn thinking_line_matches_grok_waiting_row() {
    let line = crate::theme::thinking_line(0.4);
    assert!(
        runie_core::labels::BRAILLE_EIGHT
            .iter()
            .any(|g| line.contains(*g)),
        "thinking line must carry a braille spinner frame, got: {line}"
    );
    assert!(
        line.contains("Waiting for response…"),
        "thinking line must use grok wording, got: {line}"
    );
    assert!(
        line.contains("0.4s"),
        "sub-10s timer keeps a decimal: {line}"
    );
    assert!(!line.contains('◐'), "static ◐ is gone: {line}");
    assert!(
        line.starts_with(runie_core::layout::GLYPH_AGENT),
        "feed row keeps the agent glyph prefix: {line}"
    );
    assert!(
        !line.contains("  "),
        "no double space after the agent glyph (GLYPH_AGENT includes its own): {line}"
    );

    let line = crate::theme::thinking_line(24.0);
    assert!(
        line.contains("24s") && !line.contains("24.0"),
        "≥10s timer drops the decimal: {line}"
    );
}

/// The waiting-row spinner is wall-clock driven: different elapsed buckets
/// yield different braille frames (~120ms per frame).
#[test]
fn thinking_line_spinner_advances_with_elapsed() {
    let early = crate::theme::thinking_line(0.24); // frame 2 → BRAILLE_EIGHT[2]
    let late = crate::theme::thinking_line(0.84); // frame 7 → BRAILLE_EIGHT[7]
    assert_ne!(early, late);
    assert!(early.contains(runie_core::labels::BRAILLE_EIGHT[2]));
    assert!(late.contains(runie_core::labels::BRAILLE_EIGHT[7]));
}

/// Verifies that all box drawing glyphs have correct values.
#[test]
fn glyph_box_drawing_constants_are_correct() {
    assert_eq!(BOX_HORIZONTAL, '─');
    assert_eq!(BOX_VERTICAL, '│');
    assert_eq!(BOX_TOP_LEFT, "┌");
    assert_eq!(BOX_TOP_RIGHT, "┐");
    assert_eq!(BOX_BOTTOM_LEFT, "└");
    assert_eq!(BOX_BOTTOM_RIGHT, "┘");
}
