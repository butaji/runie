//! Layer 1 + Layer 3 tests for input box scrolling and cursor visibility

use crate::event::Event;
use crate::model::AppState;

// ── Layer 1: Pure logic tests ──────────────────────────────────────────────

#[test]
fn input_scroll_starts_at_zero() {
    let state = AppState::default();
    assert_eq!(state.input.input_scroll, 0);
}

#[test]
fn newline_increases_input_scroll_to_keep_cursor_visible() {
    let mut state = AppState::default();
    // Fill input with 20 newlines (21 lines total)
    for _ in 0..20 {
        state.update(Event::Newline);
    }
    // Cursor is on line 20, but visible height is ~8 (10 height - 2 borders)
    // Scroll should have adjusted to keep cursor visible
    assert!(
        state.input.input_scroll > 0,
        "Scroll should adjust when cursor goes below visible area"
    );
    // Cursor line (20) should be within visible window
    let visible_height = 8usize; // 10 max - 2 borders
    assert!(
        state.input.input_scroll + visible_height > 20,
        "Cursor line 20 should be visible: scroll={}, visible_height={}",
        state.input.input_scroll,
        visible_height
    );
}

#[test]
fn cursor_up_scrolls_up_when_above_visible_window() {
    let mut state = AppState::default();
    // Create 15 lines
    for _ in 0..14 {
        state.update(Event::Newline);
        state.update(Event::Input('x'));
    }
    // Now cursor is at end. Scroll should be > 0
    let scroll_before = state.input.input_scroll;
    assert!(scroll_before > 0, "Should have scrolled down");
    // Move cursor to start
    state.input.cursor_pos = 0;
    state.update(Event::CursorStart);
    // Scroll should adjust up to show cursor at top
    assert_eq!(
        state.input.input_scroll, 0,
        "Should scroll to top when cursor moves to first line"
    );
}

// ── Ctrl+C behavior tests ──────────────────────────────────────────────────

#[test]
fn ctrl_c_with_empty_input_quits() {
    let mut state = AppState::default();
    assert!(state.input.input.is_empty());
    state.update(Event::Quit);
    assert!(state.should_quit, "Ctrl+C with empty input should quit");
}

#[test]
fn ctrl_c_with_non_empty_input_clears_input() {
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    assert_eq!(state.input.input, "hi");
    state.update(Event::Quit);
    assert!(
        !state.should_quit,
        "Ctrl+C with non-empty input should NOT quit"
    );
    assert!(state.input.input.is_empty(), "Input should be cleared");
    assert_eq!(state.input.cursor_pos, 0, "Cursor should reset to 0");
}

#[test]
fn ctrl_c_clears_undo_redo_stacks() {
    let mut state = AppState::default();
    state.update(Event::Input('a'));
    state.update(Event::Input('b'));
    assert!(!state.input.undo_stack.is_empty());
    state.update(Event::Quit);
    assert!(
        state.input.undo_stack.is_empty(),
        "Undo stack should be cleared"
    );
    assert!(
        state.input.redo_stack.is_empty(),
        "Redo stack should be cleared"
    );
}

#[test]
fn slash_quit_still_works_when_input_has_quit_command() {
    let mut state = AppState::default();
    // Type /quit and submit — this is different from Ctrl+C
    // The submit handler processes the slash command and then input is cleared
    state.update(Event::Input('/'));
    state.update(Event::Input('q'));
    state.update(Event::Input('u'));
    state.update(Event::Input('i'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);
    // After submit, input is empty, so Quit event should work
    assert!(state.should_quit, "/quit command should still quit app");
}
