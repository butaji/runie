//! Dirty flag regression tests.
//!
//! These tests verify the pattern that prevents the bug where calling
//! runie_tui::update() (free function) instead of tui.update() (method)
//! causes state updates without setting dirty, resulting in blank renders.

use crate::tui::state::{AppState, Msg};
use crate::tui::update::update;

/// Mock Tui struct for testing the dirty flag pattern.
/// Mirrors the exact structure of Tui.update() behavior:
/// - Sets dirty=true BEFORE calling the reducer
/// - Then calls the free function to update state
struct MockTui {
    state: AppState,
    dirty: bool,
}

impl MockTui {
    fn new(initial_dirty: bool) -> Self {
        Self {
            state: AppState::default(),
            dirty: initial_dirty,
        }
    }

    /// This is the CORRECT pattern - sets dirty BEFORE calling reducer.
    /// The bug was calling runie_tui::update() directly on state without
    /// setting dirty, causing render() to skip since !dirty.
    fn update(&mut self, msg: Msg) -> Vec<crate::tui::state::Cmd> {
        self.dirty = true;  // <-- This is the critical line that was missing!
        update(&mut self.state, msg)
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns true if render would actually draw (not skip)
    fn render(&mut self) -> bool {
        if !self.dirty {
            return false;  // Early return - render skipped
        }
        self.dirty = false;
        true  // Would actually render
    }
}

#[test]
fn test_update_sets_dirty() {
    // Bug scenario: if someone calls runie_tui::update() directly on state,
    // dirty would NOT be set. This test verifies the tui.update() pattern works.
    let mut tui = MockTui::new(false);
    assert!(!tui.is_dirty());

    tui.update(Msg::InsertChar('a'));

    assert!(tui.is_dirty(), "tui.update() must set dirty=true");
}

#[test]
fn test_render_skips_when_not_dirty() {
    let mut tui = MockTui::new(false);

    // With dirty=false, render should return early (not actually draw)
    let did_render = tui.render();
    assert!(!did_render, "render() should skip when dirty=false");
    assert!(!tui.is_dirty(), "dirty should remain false after skipped render");
}

#[test]
fn test_render_executes_when_dirty() {
    let mut tui = MockTui::new(true);

    // With dirty=true, render should execute
    let did_render = tui.render();
    assert!(did_render, "render() should execute when dirty=true");
    assert!(!tui.is_dirty(), "dirty should be cleared after render");
}

#[test]
fn test_insert_char_updates_state_and_sets_dirty() {
    let mut tui = MockTui::new(false);

    tui.update(Msg::InsertChar('x'));

    assert_eq!(tui.state.input_lines[0], "x", "InsertChar should update state");
    assert!(tui.is_dirty(), "InsertChar should set dirty=true");
}

#[test]
fn test_submit_clears_input_and_sets_dirty() {
    let mut tui = MockTui::new(false);

    // Pre-populate with "hello"
    tui.update(Msg::InsertChar('h'));
    tui.update(Msg::InsertChar('e'));
    tui.update(Msg::InsertChar('l'));
    tui.update(Msg::InsertChar('l'));
    tui.update(Msg::InsertChar('o'));

    tui.update(Msg::Submit);

    assert!(tui.state.input_lines.is_empty() || tui.state.input_lines[0].is_empty(),
            "Submit should clear input");
    assert!(tui.is_dirty(), "Submit should set dirty=true");
}

#[test]
fn test_keyboard_event_full_pipeline() {
    use crossterm::event::{Event, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};
    use crate::tui::events::event_to_msg;

    let mut tui = MockTui::new(false);

    // Simulate keyboard event: pressing 'a'
    let event = Event::Key(crossterm::event::KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    // Convert event to msg via the event_to_msg function
    if let Some(msg) = event_to_msg(event, &tui.state) {
        // This is the CORRECT path: tui.update(msg) sets dirty first
        tui.update(msg);
    }

    assert!(tui.is_dirty(), "Keyboard event pipeline should set dirty");
    assert_eq!(tui.state.input_lines[0], "a", "Keyboard event should update state");
}

// ─── Anti-pattern verification tests ─────────────────────────────────────────
// These tests document the WRONG way to update Tui state.
// If you call the free function directly on state (without setting dirty),
// the render will be skipped, causing the "typing but nothing displayed" bug.

#[test]
fn test_free_function_does_not_set_dirty() {
    // This demonstrates WHY you must use tui.update() not runie_tui::update()
    // The free function only updates state - it cannot set dirty on Tui
    let mut state = AppState::default();

    // Calling free function directly on state
    update(&mut state, Msg::InsertChar('x'));

    // State is updated correctly...
    assert_eq!(state.input_lines[0], "x");

    // ...but there's NO dirty flag mechanism in the free function
    // This is why calling it directly on a Tui's state causes the bug:
    // Tui.dirty remains false, so render() returns early!
}

#[test]
fn test_tui_update_must_be_used_not_free_function() {
    // This test verifies the contract: tui.update() is the ONLY safe way
    // to update state when using Tui. Calling the free function directly
    // bypasses the dirty flag mechanism.

    let mut tui = MockTui::new(false);

    // CORRECT: Use tui.update()
    tui.update(Msg::InsertChar('a'));
    assert!(tui.is_dirty());

    // If someone mistakenly does this:
    //   runie_tui::update(&mut tui.state, Msg::InsertChar('b'));
    // The state WOULD update, but dirty would NOT be set!
    // This is the bug we're preventing.

    // Verify clean state after correct usage
    assert_eq!(tui.state.input_lines[0], "a");
}

// ─── Forbidden pattern test ─────────────────────────────────────────────────
// This test reads tui_run.rs source and fails if it finds the forbidden
// pattern: runie_tui::update(&mut tui.state
// This prevents the regression where calling the free function directly
// on tui.state bypasses the dirty flag mechanism.

#[test]
fn test_no_direct_update_call_on_tui_state() {
    // This test reads the tui_run.rs source file and checks for the forbidden
    // pattern that causes the "typing but nothing displayed" bug.
    //
    // The forbidden pattern is:
    //   runie_tui::update(&mut tui.state, ...)
    //
    // The correct pattern is:
    //   tui.update(...)  // which sets dirty=true BEFORE calling reducer

    let tui_run_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("runie-cli")
        .join("src")
        .join("tui_run.rs");

    let source = std::fs::read_to_string(&tui_run_path)
        .expect("Failed to read tui_run.rs - may have moved");

    // Check for the forbidden pattern: runie_tui::update(&mut tui.state
    // This pattern bypasses the dirty flag mechanism in Tui.update()
    let forbidden_pattern = "runie_tui::update(&mut tui.state";

    assert!(
        !source.contains(forbidden_pattern),
        "FORBIDDEN PATTERN DETECTED: '{}' found in tui_run.rs\n\
         This bypasses Tui.update() which sets dirty=true before calling reducer.\n\
         Use tui.update(msg) instead of runie_tui::update(&mut tui.state, msg)",
        forbidden_pattern
    );

    // Also check for the re-exported update: use runie_tui::update
    // followed by direct update(&mut tui.state, ...) call
    let direct_update_patterns = [
        "update(&mut tui.state,",
        "update(&mut self.state,",
    ];

    for pattern in direct_update_patterns {
        assert!(
            !source.contains(pattern),
            "FORBIDDEN PATTERN DETECTED: '{}' found in tui_run.rs\n\
             This bypasses the dirty flag mechanism.\n\
             Use self.update(msg) or tui.update(msg) instead.",
            pattern
        );
    }
}

// ─── All update paths set dirty ─────────────────────────────────────────────
// Verifies that ALL Msg variants result in dirty=true

#[test]
fn test_all_update_paths_set_dirty() {
    let mut tui = MockTui::new(false);

    // List of Msg variants that should ALL set dirty=true
    let test_cases: Vec<(Msg, &str)> = vec![
        (Msg::InsertChar('x'), "InsertChar"),
        (Msg::Backspace, "Backspace"),
        (Msg::InsertNewline, "InsertNewline"),
        (Msg::MoveCursorLeft, "MoveCursorLeft"),
        (Msg::MoveCursorRight, "MoveCursorRight"),
        (Msg::MoveCursorToStart, "MoveCursorToStart"),
        (Msg::MoveCursorToEnd, "MoveCursorToEnd"),
        (Msg::DeleteForward, "DeleteForward"),
        (Msg::DeleteWordBackward, "DeleteWordBackward"),
        (Msg::DeleteToStart, "DeleteToStart"),
        (Msg::ToggleSidebar, "ToggleSidebar"),
        (Msg::OpenCommandPalette, "OpenCommandPalette"),
        (Msg::CloseModal, "CloseModal"),
        (Msg::Submit, "Submit"),
        (Msg::Tick, "Tick"),
        (Msg::CursorBlink, "CursorBlink"),
    ];

    for (msg, name) in test_cases {
        tui.dirty = false; // Reset dirty flag
        tui.update(msg.clone());
        assert!(
            tui.is_dirty(),
            "{} should set dirty=true but didn't",
            name
        );
    }
}

// ─── Critical difference: free function vs method ───────────────────────────
// This test documents the CRITICAL difference between:
//   - tui.update(msg)  → sets dirty=true, then updates state (CORRECT)
//   - update(&mut tui.state, msg)  → updates state but NOT dirty (BUG!)

#[test]
fn test_free_function_vs_method_difference() {
    // Demonstrate the bug: calling free function directly on state
    // does NOT set dirty, so render() skips!

    // Using the method (CORRECT)
    let mut tui = MockTui::new(false);
    tui.update(Msg::InsertChar('x'));
    assert!(tui.is_dirty(), "Method tui.update() sets dirty");
    assert_eq!(tui.state.input_lines[0], "x", "State is updated");

    // Using the free function directly on state (BUG!)
    let mut state = AppState::default();
    update(&mut state, Msg::InsertChar('y'));
    assert_eq!(state.input_lines[0], "y", "Free function updates state");

    // But there's NO dirty flag on the free function!
    // This is why you MUST use tui.update() not runie_tui::update()
    //
    // If you mistakenly do:
    //   runie_tui::update(&mut tui.state, msg);
    // The state WOULD update, but dirty would NOT be set!
    // This is why calling it directly on a Tui's state causes the bug:
    // Tui.dirty remains false, so render() returns early!
}
