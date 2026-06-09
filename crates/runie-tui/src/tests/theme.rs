//! Tests for the opaline-based theme system

use ratatui::style::Style;
use crate::theme::{
    set_current_theme, current_theme, list_builtin_themes,
    style_user, style_code_block, style_border, style_status_active,
    color_accent, color_success,
};

// ─── Layer 1: State/Logic ───────────────────────────────────────────────

#[test]
fn theme_loads_builtin_by_name() {
    set_current_theme("dracula");
    let theme = current_theme();
    assert!(!theme.meta.name.is_empty(), "Theme should have a name");
}

#[test]
fn theme_fallback_on_invalid_name() {
    set_current_theme("definitely-not-a-real-theme-12345");
    let theme = current_theme();
    // Should fall back to default (silkcircuit-neon)
    assert!(
        theme.meta.name.to_lowercase().contains("silkcircuit") || theme.meta.name.to_lowercase().contains("neon"),
        "Invalid theme should fall back to default, got: {}",
        theme.meta.name
    );
}

#[test]
fn theme_registers_runie_styles() {
    set_current_theme("silkcircuit-neon");
    let theme = current_theme();
    assert!(
        theme.has_style("runie.user"),
        "runie.user style should be registered"
    );
    assert!(
        theme.has_style("runie.agent"),
        "runie.agent style should be registered"
    );
    assert!(
        theme.has_style("runie.border"),
        "runie.border style should be registered"
    );
    assert!(
        theme.has_style("runie.code.block"),
        "runie.code.block style should be registered"
    );
}

#[test]
fn theme_style_returns_ratatui_style() {
    set_current_theme("silkcircuit-neon");
    let theme = current_theme();
    let opaline_style = theme.style("runie.user");
    let ratatui_style: Style = opaline_style.into();
    assert!(ratatui_style.fg.is_some(), "Converted style should have fg color");
}

#[test]
fn theme_builtin_list_is_nonempty() {
    let names = list_builtin_themes();
    assert!(!names.is_empty(), "Should have builtin themes");
    assert!(names.contains(&"dracula"), "Should include dracula");
    assert!(names.contains(&"nord"), "Should include nord");
}

#[test]
fn theme_changes_colors_between_themes() {
    set_current_theme("silkcircuit-neon");
    let neon_accent = color_accent();

    set_current_theme("dracula");
    let dracula_accent = color_accent();

    // Different themes should produce different accent colors
    // (If they happen to be the same, the test would be a false negative,
    // but that's extremely unlikely for these two distinct themes.)
    assert_ne!(
        neon_accent, dracula_accent,
        "Different themes should have different accent colors"
    );
}

// ─── Layer 3: Rendering (style function smoke tests) ────────────────────

#[test]
fn theme_changes_user_message_color() {
    set_current_theme("silkcircuit-neon");
    let user_style = style_user();
    assert!(user_style.fg.is_some(), "User style should have a color");
}

#[test]
fn theme_changes_code_block_bg() {
    set_current_theme("silkcircuit-neon");
    let code_style = style_code_block();
    assert!(code_style.bg.is_some(), "Code block style should have background");
}

#[test]
fn theme_changes_border_color() {
    set_current_theme("silkcircuit-neon");
    let border_style = style_border();
    assert!(border_style.fg.is_some(), "Border style should have a color");
}

#[test]
fn theme_status_active_has_success_color() {
    set_current_theme("silkcircuit-neon");
    let active = style_status_active();
    let success = color_success();
    assert_eq!(
        active.fg, Some(success),
        "Active status should use success color"
    );
}

