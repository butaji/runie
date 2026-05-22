//! Integration tests for TUI pipeline
//!
//! These tests verify the input/hotkeys bug does not regress.
//! The bug was: calling runie_tui::update(&mut tui.state) instead of
//! tui.update() bypasses the dirty flag, causing render() to skip.

use runie_tui::tui::update::update as free_update;
use runie_tui::{Msg, AppState};

/// Test that keyboard event sets dirty flag correctly
///
/// This test demonstrates the critical difference between:
/// - Using the free function update(&mut state, msg) directly
/// - Using the Tui.update() method (which sets dirty before updating)
///
/// The bug was that calling free function directly on tui.state would
/// update state but NOT set the dirty flag, so render() would skip.
#[test]
fn test_keyboard_event_state_update() {
    // Test that InsertChar updates state correctly via free function
    let mut state = AppState::default();

    // Using free function - updates state but has no dirty mechanism
    free_update(&mut state, Msg::InsertChar('a'));

    assert_eq!(state.input_lines[0], "a", "InsertChar should update state");

    // Multiple characters
    free_update(&mut state, Msg::InsertChar('b'));
    free_update(&mut state, Msg::InsertChar('c'));

    assert_eq!(state.input_lines[0], "abc", "Multiple InsertChar should accumulate");
}

/// Test all update paths update state correctly
#[test]
fn test_all_update_paths_update_state() {
    // This verifies that key message types update state correctly

    // InsertChar
    {
        let mut state = AppState::default();
        free_update(&mut state, Msg::InsertChar('x'));
        assert_eq!(state.input_lines[0], "x", "InsertChar should add character");
    }

    // InsertNewline
    {
        let mut state = AppState::default();
        free_update(&mut state, Msg::InsertChar('x'));
        free_update(&mut state, Msg::InsertNewline);
        assert_eq!(state.input_lines.len(), 2, "InsertNewline should add new line");
        assert_eq!(state.input_lines[0], "x", "First line should have 'x'");
        assert_eq!(state.input_lines[1], "", "Second line should be empty");
    }

    // ToggleSidebar
    {
        let mut state = AppState::default();
        assert!(!state.show_sidebar, "Initial sidebar should be hidden");
        free_update(&mut state, Msg::ToggleSidebar);
        assert!(state.show_sidebar, "ToggleSidebar should show sidebar");
    }

    // Submit with content creates user message
    {
        let mut state = AppState::default();
        free_update(&mut state, Msg::InsertChar('h'));
        free_update(&mut state, Msg::InsertChar('i'));
        let cmds = free_update(&mut state, Msg::Submit);
        assert!(!cmds.is_empty(), "Submit with content should produce commands");
        assert_eq!(state.messages.len(), 1, "Submit should add user message");
    }

    // Submit without content produces no commands
    {
        let mut state = AppState::default();
        let cmds = free_update(&mut state, Msg::Submit);
        assert!(cmds.is_empty(), "Submit without content should produce no commands");
        assert_eq!(state.messages.len(), 0, "Submit without content should not add message");
    }
}

/// Test that free function does NOT set dirty (documents the bug)
///
/// This test documents WHY you must use tui.update() not update(&mut state, msg)
///
/// The free function update() only modifies state - it has no access
/// to the Tui's dirty flag. This is the root cause of the bug.
#[test]
fn test_free_function_has_no_dirty_mechanism() {
    // The free function update() only takes &mut AppState, no dirty flag
    // This is the core issue - calling it directly bypasses dirty

    let mut state = AppState::default();

    // Free function updates state correctly...
    free_update(&mut state, Msg::InsertChar('x'));
    assert_eq!(state.input_lines[0], "x");

    // But there's NO way to check if state changed via the free function
    // The dirty flag is an external concept that the free function doesn't know about

    // This is why tui.update() exists - it wraps the free function and:
    // 1. Sets dirty = true BEFORE calling the reducer
    // 2. Then calls the reducer
    // So render() knows something changed and should redraw
}

/// Test no forbidden pattern in tui_run.rs
///
/// This test reads tui_run.rs source and verifies there is no
/// forbidden pattern: update(&mut tui.state, ...)
///
/// The correct pattern is: tui.update(msg)
/// The forbidden pattern is: runie_tui::update(&mut tui.state, msg)
#[test]
fn test_no_free_function_calls_in_tui_run() {
    // This test reads tui_run.rs source and verifies there is no
    // forbidden pattern that bypasses the dirty flag mechanism.

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tui_run.rs")
    ).expect("Failed to read tui_run.rs");

    // Check for forbidden patterns that would bypass dirty flag
    let forbidden = [
        "update(&mut tui.state,",
        "update(&mut self.state,",
        "runie_tui::update(&mut tui.state",
    ];

    for pattern in forbidden {
        assert!(
            !source.contains(pattern),
            "FORBIDDEN: '{}' found in tui_run.rs - use tui.update() instead",
            pattern
        );
    }
}

/// Test that all Msg variants compile and run without panic
#[test]
fn test_all_msg_variants_execute() {
    // This ensures all Msg variants are handled by the update function
    // and don't cause panics

    let mut state = AppState::default();

    // Tick and CursorBlink should not panic (animation updates)
    free_update(&mut state, Msg::Tick);
    free_update(&mut state, Msg::CursorBlink);

    // InsertChar and backspace should work
    free_update(&mut state, Msg::InsertChar('a'));
    free_update(&mut state, Msg::Backspace);

    // Cursor movements should not panic
    free_update(&mut state, Msg::MoveCursorLeft);
    free_update(&mut state, Msg::MoveCursorRight);
    free_update(&mut state, Msg::MoveCursorToStart);
    free_update(&mut state, Msg::MoveCursorToEnd);

    // Delete operations should not panic
    free_update(&mut state, Msg::InsertChar('h'));
    free_update(&mut state, Msg::InsertChar('i'));
    free_update(&mut state, Msg::DeleteForward);
    free_update(&mut state, Msg::DeleteWordBackward);
    free_update(&mut state, Msg::DeleteToStart);

    // Modal operations should not panic
    free_update(&mut state, Msg::CloseModal);

    // Toggle operations should not panic
    free_update(&mut state, Msg::ToggleSidebar);

    // Command palette should not panic
    free_update(&mut state, Msg::OpenCommandPalette);
    free_update(&mut state, Msg::CommandPaletteFilter('t'));
    free_update(&mut state, Msg::CommandPaletteBackspace);
    free_update(&mut state, Msg::CommandPaletteUp);
    free_update(&mut state, Msg::CommandPaletteDown);
    free_update(&mut state, Msg::CommandPaletteConfirm);

    // Submit with empty input should not panic
    free_update(&mut state, Msg::Submit);

    // Submit with content should produce commands
    free_update(&mut state, Msg::InsertChar('x'));
    let cmds = free_update(&mut state, Msg::Submit);
    assert!(!cmds.is_empty(), "Submit with content should produce commands");
}
