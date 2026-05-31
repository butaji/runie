//! Tests for Chat ↔ CommandPalette transitions.

use super::*;

/// Test: Chat → CommandPalette via OpenCommandPalette.
#[test]
fn test_chat_to_palette() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Start in Chat
    assert_eq!(state.mode, TuiMode::Chat);

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);
    assert!(state.command_palette.open);
}

/// Test: CommandPalette → Chat via CloseModal.
#[test]
fn test_palette_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Start in palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Close palette
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(!state.command_palette.open);
}

/// Test: Chat → CommandPalette → Chat round-trip.
#[test]
fn test_chat_palette_chat_roundtrip() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Chat
    assert_eq!(state.mode, TuiMode::Chat);

    // To palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Back to chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Palette accepts Ctrl+K to open.
#[test]
fn test_palette_ctrl_k_opens_palette() {
    let state = make_state();
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::OpenCommandPalette));
}

/// Test: Esc closes palette (via CommandPaletteCancelArgument when not in argument mode).
#[test]
fn test_esc_closes_palette() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Esc in palette sends CancelArgument
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Enter in palette confirms selection.
#[test]
fn test_enter_confirms_in_palette() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Enter in palette mode produces CommandPaletteConfirm
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteConfirm));
}

/// Test: Up/Down navigate palette.
#[test]
fn test_up_down_navigate_palette() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Up
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteUp));

    // Down
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteDown));
}

/// Test: palette filter via character input.
#[test]
fn test_palette_filter_char() {
    let state = make_state();
    let msg = simulate_key(KeyCode::Char('h'), KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteFilter('h')));
}

/// Test: palette backspace.
#[test]
fn test_palette_backspace() {
    let state = make_state();
    let msg = simulate_key(KeyCode::Backspace, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteBackspace));
}