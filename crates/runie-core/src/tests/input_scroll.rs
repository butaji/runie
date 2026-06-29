//! Layer 1 + Layer 3 tests for input box scrolling and cursor visibility

use crate::model::AppState;
use crate::Event;

// ── Layer 1: Pure logic tests ──────────────────────────────────────────────

#[test]
fn input_scroll_starts_at_zero() {
    let state = AppState::default();
    assert_eq!(state.input.input_scroll, 0);
}

/// Layer 1: Scroll adjustment only happens during cursor navigation, not content changes.
#[test]
fn input_scroll_not_adjusted_on_content_changes() {
    let mut state = AppState::default();
    // Fill input with 20 newlines (21 lines total)
    for _ in 0..20 {
        state.update(crate::Event::Newline);
    }
    // Scroll stays at 0 on content changes - this is the actual behavior
    assert_eq!(
        state.input.input_scroll, 0,
        "Scroll stays at 0 on content change"
    );
}

/// Layer 1: Cursor navigation does clamp input_scroll to keep cursor visible.
#[test]
fn cursor_nav_clamps_input_scroll_when_cursor_below_visible() {
    let mut state = AppState::default();
    // Create 15 lines
    for _ in 0..14 {
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('x'));
    }
    // Scroll stays at 0 after content changes
    assert_eq!(state.input.input_scroll, 0, "Scroll starts at 0");
    // Move cursor to end - clamp should adjust scroll (cursor is on line 14, visible is 8)
    state.update(crate::Event::CursorEnd);
    // After moving to end, scroll should be adjusted so cursor is visible
    // Line 14 should be visible with visible_height=8: scroll should be 14-7=7
    let visible_height = 8usize; // MAX_INPUT_HEIGHT(10) - BORDER_ROWS(2)
    let cursor_line = 14; // 15 total lines (0-14)
    let expected_scroll = (cursor_line as usize).saturating_sub(visible_height - 1);
    assert_eq!(
        state.input.input_scroll, expected_scroll,
        "Scroll should clamp to keep cursor line {} visible (expected {})",
        cursor_line, expected_scroll
    );
}

/// Layer 1: Cursor to start adjusts scroll to 0.
#[test]
fn cursor_start_resets_scroll_to_zero() {
    let mut state = AppState::default();
    // Create 15 lines
    for _ in 0..14 {
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('x'));
    }
    // Move to end (scroll gets clamped)
    state.update(crate::Event::CursorEnd);
    assert!(
        state.input.input_scroll > 0,
        "Scroll should be > 0 after CursorEnd"
    );
    // Move to start - scroll should reset to 0
    state.update(crate::Event::CursorStart);
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
    state.update(crate::Event::Quit);
    assert!(state.should_quit, "Ctrl+C with empty input should quit");
}

#[test]
fn ctrl_c_with_non_empty_input_clears_input() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    assert_eq!(state.input.input, "hi");
    state.update(crate::Event::Quit);
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
    state.update(crate::Event::Input('a'));
    state.update(crate::Event::Input('b'));
    assert!(!state.input.undo_stack.is_empty());
    state.update(crate::Event::Quit);
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
    state.update(crate::Event::Input('/'));
    state.update(crate::Event::Input('q'));
    state.update(crate::Event::Input('u'));
    state.update(crate::Event::Input('i'));
    state.update(crate::Event::Input('t'));
    state.update(Event::submit());
    // After submit, input is empty, so Quit event should work
    assert!(state.should_quit, "/quit command should still quit app");
}
