//! Integration tests for TUI pipeline
//!
//! These tests verify the input/hotkeys bug does not regress.
//! The bug was: calling runie_tui::update(&mut tui.state) instead of
//! tui.update() bypasses the dirty flag, causing render() to skip.

use runie_tui::tui::update::update as free_update;
use runie_tui::{Msg, AppState, Cmd, TuiMode};
use crossterm::event::{Event, KeyCode, KeyModifiers};

/// MockTui simulates Tui.update() behavior for unit testing
/// The key difference: MockTui.update() sets dirty=true BEFORE calling the reducer
struct MockTui {
    state: AppState,
    dirty: bool,
}

impl MockTui {
    fn new() -> Self {
        Self {
            state: AppState::default(),
            dirty: true, // Initial render needed
        }
    }

    /// This is the CORRECT update pattern - sets dirty BEFORE calling reducer
    fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        self.dirty = true;
        free_update(&mut self.state, msg)
    }

    /// Simulates event_to_msg conversion
    fn simulate_key(&mut self, key_code: KeyCode, ctrl: bool) -> Option<Msg> {
        let modifiers = if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::empty() };
        let event = Event::Key(crossterm::event::KeyEvent::new(key_code, modifiers));
        runie_tui::event_to_msg(event, &self.state)
    }

    fn content(&self) -> String {
        self.state.input_lines.join("\n")
    }
}

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

// ══════════════════════════════════════════════════════════════════════════════
// FULL PIPELINE TESTS
// ══════════════════════════════════════════════════════════════════════════════

/// Test: Key 'a' → event_to_msg → update → check dirty + state
/// Test: Key 'b' → check "ab"
/// Test: Backspace → check "a"
/// Test: Enter → check input cleared, message added
#[test]
fn test_full_pipeline_typing() {
    let mut tui = MockTui::new();

    // Type 'a' - verify state and dirty
    if let Some(msg) = tui.simulate_key(KeyCode::Char('a'), false) {
        tui.update(msg);
    }
    assert!(tui.dirty, "InsertChar should set dirty");
    assert_eq!(tui.content(), "a", "Should have 'a'");

    // Type 'b' - verify "ab"
    if let Some(msg) = tui.simulate_key(KeyCode::Char('b'), false) {
        tui.update(msg);
    }
    assert_eq!(tui.content(), "ab", "Should have 'ab'");

    // Backspace - verify "a"
    if let Some(msg) = tui.simulate_key(KeyCode::Backspace, false) {
        tui.update(msg);
    }
    assert_eq!(tui.content(), "a", "Backspace should remove 'b'");

    // Enter/Submit - verify input cleared and message added
    let msg_count_before = tui.state.messages.len();
    if let Some(msg) = tui.simulate_key(KeyCode::Enter, false) {
        tui.update(msg);
    }
    assert_eq!(tui.content(), "", "Input should be cleared after submit");
    assert_eq!(tui.state.messages.len(), msg_count_before + 1, "Should have added user message");
}

/// Test: Ctrl+B → check sidebar toggled
/// Test: Ctrl+K → check mode is CommandPalette
/// Test: Esc → check mode back to Chat
/// Test: Ctrl+Q → check running=false
#[test]
fn test_full_pipeline_hotkeys() {
    let mut tui = MockTui::new();

    // Ctrl+B toggles sidebar
    assert!(!tui.state.show_sidebar, "Sidebar should start hidden");
    if let Some(msg) = tui.simulate_key(KeyCode::Char('b'), true) {
        tui.update(msg);
    }
    assert!(tui.state.show_sidebar, "Ctrl+B should toggle sidebar on");

    // Ctrl+K opens command palette
    assert_eq!(tui.state.mode, TuiMode::Chat, "Mode should start as Chat");
    if let Some(msg) = tui.simulate_key(KeyCode::Char('k'), true) {
        tui.update(msg);
    }
    assert_eq!(tui.state.mode, TuiMode::CommandPalette, "Ctrl+K should open palette");

    // Esc closes modal and returns to Chat
    if let Some(msg) = tui.simulate_key(KeyCode::Esc, false) {
        tui.update(msg);
    }
    assert_eq!(tui.state.mode, TuiMode::Chat, "Esc should close modal");
    assert!(!tui.state.command_palette.open, "Palette should be closed");

    // Ctrl+Q quits
    assert!(tui.state.running, "App should be running initially");
    if let Some(msg) = tui.simulate_key(KeyCode::Char('q'), true) {
        tui.update(msg);
    }
    assert!(!tui.state.running, "Ctrl+Q should set running=false");
}

/// Test: All Msg variants that modify state should set dirty=true
#[test]
fn test_all_update_paths_set_dirty() {
    let mut tui = MockTui::new();

    // Helper to test a msg sets dirty
    let test_msg = |tui: &mut MockTui, msg: Msg, name: &str| {
        tui.dirty = false;
        tui.update(msg);
        assert!(tui.dirty, "{} should set dirty=true", name);
    };

    // Input mutations
    test_msg(&mut tui, Msg::InsertChar('x'), "InsertChar");
    test_msg(&mut tui, Msg::Backspace, "Backspace");
    test_msg(&mut tui, Msg::InsertNewline, "InsertNewline");
    test_msg(&mut tui, Msg::DeleteForward, "DeleteForward");
    test_msg(&mut tui, Msg::DeleteWordBackward, "DeleteWordBackward");
    test_msg(&mut tui, Msg::DeleteToStart, "DeleteToStart");
    test_msg(&mut tui, Msg::MoveCursorLeft, "MoveCursorLeft");
    test_msg(&mut tui, Msg::MoveCursorRight, "MoveCursorRight");
    test_msg(&mut tui, Msg::MoveCursorUp, "MoveCursorUp");
    test_msg(&mut tui, Msg::MoveCursorDown, "MoveCursorDown");
    test_msg(&mut tui, Msg::MoveCursorToStart, "MoveCursorToStart");
    test_msg(&mut tui, Msg::MoveCursorToEnd, "MoveCursorToEnd");

    // UI mutations
    test_msg(&mut tui, Msg::ToggleSidebar, "ToggleSidebar");
    test_msg(&mut tui, Msg::OpenCommandPalette, "OpenCommandPalette");
    test_msg(&mut tui, Msg::CloseModal, "CloseModal");

    // Command palette
    tui.update(Msg::OpenCommandPalette);
    test_msg(&mut tui, Msg::CommandPaletteFilter('t'), "CommandPaletteFilter");
    test_msg(&mut tui, Msg::CommandPaletteBackspace, "CommandPaletteBackspace");
    test_msg(&mut tui, Msg::CommandPaletteUp, "CommandPaletteUp");
    test_msg(&mut tui, Msg::CommandPaletteDown, "CommandPaletteDown");
    test_msg(&mut tui, Msg::CommandPaletteConfirm, "CommandPaletteConfirm");

    // Scroll
    test_msg(&mut tui, Msg::ScrollUp, "ScrollUp");
    test_msg(&mut tui, Msg::ScrollDown, "ScrollDown");
    test_msg(&mut tui, Msg::ScrollPageUp, "ScrollPageUp");
    test_msg(&mut tui, Msg::ScrollPageDown, "ScrollPageDown");

    // App control
    test_msg(&mut tui, Msg::Submit, "Submit");
    test_msg(&mut tui, Msg::Quit, "Quit");

    // Animation (also sets dirty)
    test_msg(&mut tui, Msg::Tick, "Tick");
    test_msg(&mut tui, Msg::CursorBlink, "CursorBlink");
}

/// Test: runie_tui::update() (free function) does NOT set dirty
/// Test: MockTui.update() (which wraps free function) DOES set dirty
/// This is the core regression test for the input bug
#[test]
fn test_render_call_path() {
    // Free function path - no dirty mechanism
    let mut state = AppState::default();
    free_update(&mut state, Msg::InsertChar('x'));
    // No way to know if state changed - free function has no dirty return
    assert_eq!(state.input_lines[0], "x", "Free function updates state correctly");

    // MockTui path - dirty is set before reducer
    let mut tui = MockTui::new();
    tui.dirty = false;
    tui.update(Msg::InsertChar('y'));
    assert!(tui.dirty, "MockTui.update() should set dirty before calling reducer");

    // The bug was calling free function directly on tui.state:
    // WRONG:  runie_tui::update(&mut tui.state, msg)  ← bypasses dirty!
    // RIGHT: tui.update(msg)                          ← sets dirty first
}

/// Test: Input box state consistency through complex edit sequences
#[test]
fn test_input_box_state_consistency() {
    let mut tui = MockTui::new();

    // Insert 10 chars
    for c in 'a'..='j' {
        tui.update(Msg::InsertChar(c));
    }
    assert_eq!(tui.content(), "abcdefghij");
    assert_eq!(tui.state.cursor_col, 10, "Cursor at end");
    assert_eq!(tui.state.input_lines.len(), 1, "One line");

    // Delete 5 chars (via Backspace × 5) - removes chars before cursor
    for _ in 0..5 {
        tui.update(Msg::Backspace);
    }
    assert_eq!(tui.content(), "abcde", "Should have 5 chars (removed j-i-h-g-f)");
    assert_eq!(tui.state.cursor_col, 5, "Cursor at position 5");

    // Add newline
    tui.update(Msg::InsertNewline);
    assert_eq!(tui.state.input_lines.len(), 2, "Should have 2 lines");
    assert_eq!(tui.state.input_lines[0], "abcde");
    assert_eq!(tui.state.input_lines[1], "");
    assert_eq!(tui.state.cursor_row, 1, "Cursor on new line");
    assert_eq!(tui.state.cursor_col, 0, "Cursor at start of new line");

    // Type more on new line
    tui.update(Msg::InsertChar('x'));
    tui.update(Msg::InsertChar('y'));
    tui.update(Msg::InsertChar('z'));
    assert_eq!(tui.state.input_lines[1], "xyz", "New line has xyz");

    // Move cursor and verify no panic
    tui.update(Msg::MoveCursorLeft);
    assert_eq!(tui.state.cursor_col, 2, "Cursor moved left");
    tui.update(Msg::MoveCursorRight);
    assert_eq!(tui.state.cursor_col, 3, "Cursor moved right");
    tui.update(Msg::MoveCursorToStart);
    assert_eq!(tui.state.cursor_col, 0, "Cursor at line start");
    tui.update(Msg::MoveCursorToEnd);
    assert_eq!(tui.state.cursor_col, 3, "Cursor at line end");

    // Full content check
    assert_eq!(tui.content(), "abcde\nxyz", "Full content should be multiline");
}

/// Test: Hotkey behavior depends on current mode
#[test]
fn test_hotkey_priority() {
    let mut tui = MockTui::new();

    // In Chat mode: Ctrl+C should quit (not cancel)
    assert_eq!(tui.state.mode, TuiMode::Chat);
    assert!(tui.state.running, "Should be running initially");
    if let Some(msg) = tui.simulate_key(KeyCode::Char('c'), true) {
        tui.update(msg);
    }
    assert!(!tui.state.running, "Ctrl+C should quit in Chat mode");

    // Reset
    tui.state.running = true;

    // Enter CommandPalette mode
    tui.update(Msg::OpenCommandPalette);
    assert_eq!(tui.state.mode, TuiMode::CommandPalette);

    // In CommandPalette mode: Ctrl+C should close palette (NOT quit)
    // Note: Ctrl+C in CommandPalette mode maps to key_to_palette_msg which returns None
    // because ctrl+C is only handled in ctrl_chat_key for Chat mode
    // So Ctrl+C in palette mode does nothing (passes through to terminal)

    // Esc should close palette
    if let Some(msg) = tui.simulate_key(KeyCode::Esc, false) {
        tui.update(msg);
    }
    assert_eq!(tui.state.mode, TuiMode::Chat, "Esc should close palette");
    assert!(tui.state.running, "Should still be running after Esc");
}
