//! Theme caching, quantization, and glyph tests.

use std::sync::Arc;

use crate::terminal::caps::{MouseCapability, TermCaps};
use crate::theme::glyph::{
    BOX_BOTTOM_LEFT, BOX_BOTTOM_RIGHT, BOX_HORIZONTAL, BOX_TOP_LEFT, BOX_TOP_RIGHT, BOX_VERTICAL, GLYPH_BULLET,
    GLYPH_CHECK, GLYPH_CHECKED, GLYPH_DOWNLOAD, GLYPH_FILTER, GLYPH_SELECTED, GLYPH_SPINNER, GLYPH_TOOL,
    GLYPH_UNCHECKED, GLYPH_UNSELECTED, GLYPH_X, INDICATOR_COLLAPSED, INDICATOR_ERROR, PANEL_CHAT, PANEL_INPUT,
    SCROLLBAR_THUMB, SCROLLBAR_TRACK,
};
use crate::theme::loader::{default_theme, minimal_fallback_theme, validate_theme};
use crate::theme::{
    current_theme, current_theme_name, set_current_theme, set_current_theme_with_caps, test_lock, BUILTIN_THEMES,
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
    TermCaps { truecolor: true, mouse: MouseCapability::Sgr, ..Default::default() }
}

fn ansi256_caps() -> TermCaps {
    TermCaps { truecolor: false, mouse: MouseCapability::Legacy, ..Default::default() }
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
fn monochrome_caps_suppress_theme_colors_but_keep_rendering_available() {
    let _lock = test_lock();
    let caps = TermCaps { color_depth: crate::terminal::caps::ColorDepth::None, ..Default::default() };
    set_current_theme_with_caps("runie", caps);
    assert!(crate::theme::is_monochrome());
    assert_eq!(crate::theme::color_accent(), ratatui::style::Color::Reset);
    assert_eq!(crate::theme::color_bg(), ratatui::style::Color::Reset);
    assert_eq!(
        crate::theme::style_agent().fg,
        Some(ratatui::style::Color::Reset)
    );
    set_current_theme_with_caps("runie", truecolor_caps());
}

#[test]
fn limited_unicode_caps_use_ascii_feed_fallbacks() {
    let _lock = test_lock();
    let caps = TermCaps { unicode: false, ..truecolor_caps() };
    set_current_theme_with_caps("runie", caps);
    assert_eq!(crate::theme::rail_glyph(), "|");
    assert_eq!(crate::theme::scrollbar_track_glyph(), "|");
    assert_eq!(crate::theme::scrollbar_thumb_glyph(), "#");
    set_current_theme_with_caps("runie", truecolor_caps());
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
    for name in BUILTIN_THEMES
        .iter()
        .copied()
        .filter(|name| !matches!(*name, "auto" | "system"))
    {
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
fn auto_theme_alias_resolves_from_appearance_hint() {
    let _lock = test_lock();
    std::env::set_var("RUNIE_THEME_APPEARANCE", "light");
    set_current_theme_with_caps("auto", truecolor_caps());
    assert_eq!(current_theme_name(), "auto");
    assert!(current_theme().color("bg.base").r > 200);
    std::env::set_var("RUNIE_THEME_APPEARANCE", "dark");
    set_current_theme_with_caps("system", truecolor_caps());
    assert!(current_theme().color("bg.base").r < 100);
    std::env::remove_var("RUNIE_THEME_APPEARANCE");
    set_current_theme_with_caps("runie", truecolor_caps());
}

#[test]
fn every_builtin_theme_supports_runie_feed_tokens() {
    let _lock = test_lock();
    let required = [
        "accent.thinking",
        "accent.plan",
        "accent.feedback",
        "accent.monitor",
        "rail.running",
        "rail.success",
        "rail.error",
        "rail.thinking",
    ];
    for name in BUILTIN_THEMES
        .iter()
        .copied()
        .filter(|name| *name != "auto" && *name != "system")
    {
        set_current_theme(name);
        let theme = current_theme();
        for token in required {
            assert!(theme.try_color(token).is_some(), "{name} missing {token}");
        }
    }
    set_current_theme("runie");
}

#[test]
fn every_builtin_theme_passes_renderer_contract() {
    let _lock = test_lock();
    for name in BUILTIN_THEMES {
        set_current_theme_with_caps(name, truecolor_caps());
        validate_theme(&current_theme()).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    set_current_theme("runie");
}

#[test]
fn every_builtin_theme_has_stable_roles_at_each_terminal_depth() {
    let _lock = test_lock();
    let caps = [
        TermCaps {
            color_depth: crate::terminal::caps::ColorDepth::Truecolor,
            truecolor: true,
            mouse: MouseCapability::Sgr,
            ..Default::default()
        },
        TermCaps {
            color_depth: crate::terminal::caps::ColorDepth::ANSI256,
            mouse: MouseCapability::Legacy,
            ..Default::default()
        },
        TermCaps {
            color_depth: crate::terminal::caps::ColorDepth::ANSI16,
            mouse: MouseCapability::None,
            ..Default::default()
        },
        TermCaps { color_depth: crate::terminal::caps::ColorDepth::None, unicode: false, ..Default::default() },
    ];
    let required = [
        "text.primary",
        "text.dim",
        "border.unfocused",
        "border.focused",
        "bg.selection",
        "rail.running",
        "rail.success",
        "rail.error",
    ];
    for name in BUILTIN_THEMES
        .iter()
        .copied()
        .filter(|name| *name != "auto" && *name != "system")
    {
        for cap in caps {
            set_current_theme_with_caps(name, cap);
            let theme = current_theme();
            for role in required {
                assert!(
                    theme.try_color(role).is_some(),
                    "{name} missing {role} at {:?}",
                    cap.color_depth
                );
            }
        }
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
    assert_eq!(
        SCROLLBAR_TRACK, "│",
        "scrollbar track should be visible vertical bar"
    );
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

/// The thinking feed item is stable; animation is owned by the live status row.
#[test]
fn thinking_line_matches_grok_waiting_row() {
    let line = crate::theme::thinking_line(0.4);
    assert_eq!(line, "Thinking…");
    assert!(!runie_core::labels::BRAILLE_EIGHT
        .iter()
        .any(|g| line.contains(*g)));
}

#[test]
fn thinking_line_header_is_not_the_animation_source() {
    let first = crate::theme::thinking_line(0.0);
    let next = crate::theme::thinking_line(0.24);
    assert_eq!(first, next);
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
