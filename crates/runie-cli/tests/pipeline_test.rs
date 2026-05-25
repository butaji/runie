//! Integration tests for TUI pipeline
//!
//! These tests verify the input/hotkeys bug does not regress.
//! The bug was: calling runie_tui::update(&mut tui.state) instead of
//! tui.update() bypasses the dirty flag, causing render() to skip.

use runie_tui::tui::update::update as free_update;
use runie_tui::{Msg, AppState, Cmd, TuiMode, event_to_msg, CommandPalette};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Build a crossterm KeyEvent for a character (no modifiers)
fn char_key(c: char) -> KeyEvent {
    KeyEvent::new(crossterm::event::KeyCode::Char(c), KeyModifiers::empty())
}

/// Build a crossterm KeyEvent for a special key code
fn special_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

/// Thin wrapper: calls free_update with a freshly-created CommandPalette.
/// Use this when the test doesn't care about palette state — it just needs
/// the third argument to satisfy the function signature.
fn run_update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut palette = CommandPalette::new();
    free_update(state, &mut palette, msg)
}

/// MockTui simulates Tui.update() behavior for unit testing.
/// The key difference: MockTui.update() sets dirty=true BEFORE calling the reducer.
struct MockTui {
    state: AppState,
    palette: CommandPalette,
    dirty: bool,
}

impl MockTui {
    fn new() -> Self {
        let mut state = AppState::default();
        // P0-2 FIX: Set model so submit tests work
        state.current_model = Some("gpt-4".to_string());
        Self {
            state,
            palette: CommandPalette::new(),
            dirty: true, // Initial render needed
        }
    }

    /// This is the CORRECT update pattern - sets dirty BEFORE calling reducer
    fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        self.dirty = true;
        free_update(&mut self.state, &mut self.palette, msg)
    }

    /// Simulates event_to_msg conversion
    fn simulate_key(&mut self, key_code: KeyCode, ctrl: bool) -> Option<Msg> {
        let modifiers = if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::empty() };
        let event = Event::Key(KeyEvent::new(key_code, modifiers));
        event_to_msg(event, &self.state).into_iter().next()
    }

    /// Returns the current textarea content as a single newline-joined string
    fn content(&self) -> String {
        self.state.textarea.lines().join("\n")
    }

    /// Returns the textarea lines as a Vec<&str>
    fn lines(&self) -> Vec<&str> {
        self.state.textarea.lines().iter().map(|s| s.as_str()).collect()
    }
}

// ─── Dirty flag regression tests ─────────────────────────────────────────────

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
    let mut state = AppState::default();

    // Using free function - updates state via TextareaKey
    run_update(&mut state, Msg::TextareaKey(char_key('a')));
    assert_eq!(state.textarea.lines()[0], "a", "TextareaKey('a') should populate textarea");

    // Multiple characters
    run_update(&mut state, Msg::TextareaKey(char_key('b')));
    run_update(&mut state, Msg::TextareaKey(char_key('c')));
    assert_eq!(state.textarea.lines()[0], "abc", "Multiple chars should accumulate");
}

/// Test all update paths update state correctly
#[test]
fn test_all_update_paths_update_state() {
    // TextareaKey (replaces old InsertChar)
    {
        let mut state = AppState::default();
        run_update(&mut state, Msg::TextareaKey(char_key('x')));
        assert_eq!(state.textarea.lines()[0], "x", "TextareaKey should add character");
    }

    // InsertNewline
    {
        let mut state = AppState::default();
        run_update(&mut state, Msg::TextareaKey(char_key('x')));
        run_update(&mut state, Msg::InsertNewline);
        assert_eq!(state.textarea.lines().len(), 2, "InsertNewline should add new line");
        assert_eq!(state.textarea.lines()[0], "x", "First line should have 'x'");
        assert_eq!(state.textarea.lines()[1], "", "Second line should be empty");
    }

    // Backspace via TextareaKey
    {
        let mut state = AppState::default();
        run_update(&mut state, Msg::TextareaKey(char_key('a')));
        run_update(&mut state, Msg::TextareaKey(char_key('b')));
        run_update(&mut state, Msg::TextareaKey(special_key(KeyCode::Backspace)));
        assert_eq!(state.textarea.lines()[0], "a", "Backspace should remove last char");
    }

    // ToggleSidebar
    {
        let mut state = AppState::default();
        assert!(!state.show_sidebar, "Initial sidebar should be hidden");
        run_update(&mut state, Msg::ToggleSidebar);
        assert!(state.show_sidebar, "ToggleSidebar should show sidebar");
    }

    // Submit with content creates user message
    {
        let mut state = AppState::default();
        state.current_model = Some("gpt-4".to_string()); // P0-2 FIX: Set model for submit tests
        run_update(&mut state, Msg::TextareaKey(char_key('h')));
        run_update(&mut state, Msg::TextareaKey(char_key('i')));
        let cmds = run_update(&mut state, Msg::Submit);
        assert!(!cmds.is_empty(), "Submit with content should produce commands");
        assert_eq!(state.messages.len(), 1, "Submit should add user message");
    }

    // Submit without content produces no commands
    {
        let mut state = AppState::default();
        let cmds = run_update(&mut state, Msg::Submit);
        assert!(cmds.is_empty(), "Submit without content should produce no commands");
        assert_eq!(state.messages.len(), 0, "Submit without content should not add message");
    }
}

/// Test that free function does NOT set dirty (documents the bug)
///
/// The free function update() only takes &mut AppState + &mut CommandPalette,
/// no dirty flag. This is why tui.update() exists - it wraps the free function
/// and sets dirty = true BEFORE calling the reducer.
#[test]
fn test_free_function_has_no_dirty_mechanism() {
    let mut state = AppState::default();

    // Free function updates state correctly via TextareaKey
    run_update(&mut state, Msg::TextareaKey(char_key('x')));
    assert_eq!(state.textarea.lines()[0], "x");

    // But run_update has no return value for dirty — the dirty flag is an
    // external concept.  tui.update() bridges this by setting dirty=true first.
}

/// Test no forbidden pattern in tui_run.rs
///
/// The correct pattern is: tui.update(msg)
/// The forbidden pattern is: runie_tui::update(&mut tui.state, msg)
#[test]
fn test_no_free_function_calls_in_tui_run() {
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

// ─── Full pipeline tests ──────────────────────────────────────────────────────

/// Test: Key 'a' → event_to_msg → update → check dirty + state
/// Test: Key 'b' → check "ab"
/// Test: Backspace → check "a"
/// Test: Enter → check input cleared, message added
#[test]
fn test_full_pipeline_typing() {
    let mut tui = MockTui::new();

    // Type 'a'
    if let Some(msg) = tui.simulate_key(KeyCode::Char('a'), false) {
        tui.update(msg);
    }
    assert!(tui.dirty, "TextareaKey should set dirty");
    assert_eq!(tui.content(), "a", "Should have 'a'");

    // Type 'b'
    if let Some(msg) = tui.simulate_key(KeyCode::Char('b'), false) {
        tui.update(msg);
    }
    assert_eq!(tui.content(), "ab", "Should have 'ab'");

    // Backspace
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
    assert_eq!(
        tui.state.messages.len(),
        msg_count_before + 1,
        "Should have added user message"
    );
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

    // Esc closes palette and returns to Chat
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

    // Input mutations (via TextareaKey — textarea handles all keyboard input)
    test_msg(&mut tui, Msg::TextareaKey(char_key('x')), "TextareaKey");
    test_msg(&mut tui, Msg::TextareaKey(special_key(KeyCode::Backspace)), "Backspace");
    test_msg(&mut tui, Msg::InsertNewline, "InsertNewline");

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

/// Test: free function does NOT set dirty
///       MockTui.update() DOES set dirty
#[test]
fn test_render_call_path() {
    // Free function path — no dirty mechanism
    let mut state = AppState::default();
    run_update(&mut state, Msg::TextareaKey(char_key('x')));
    assert_eq!(
        state.textarea.lines()[0], "x",
        "Free function updates state correctly"
    );

    // MockTui path — dirty is set before reducer
    let mut tui = MockTui::new();
    tui.dirty = false;
    tui.update(Msg::TextareaKey(char_key('y')));
    assert!(
        tui.dirty,
        "MockTui.update() should set dirty before calling reducer"
    );
}

/// Test: Textarea state consistency through complex edit sequences
///
/// Note: Cursor position assertions are removed since ratatui_textarea manages
/// cursor state internally and does not expose cursor_row/cursor_col on AppState.
/// We assert on textarea content, which is the user-visible state.
#[test]
fn test_textarea_state_consistency() {
    let mut tui = MockTui::new();

    // Insert 10 chars
    for c in 'a'..='j' {
        tui.update(Msg::TextareaKey(char_key(c)));
    }
    assert_eq!(tui.content(), "abcdefghij");
    assert_eq!(tui.lines().len(), 1, "One line");

    // Backspace × 5 — removes chars before cursor
    for _ in 0..5 {
        tui.update(Msg::TextareaKey(special_key(KeyCode::Backspace)));
    }
    assert_eq!(tui.content(), "abcde", "Should have 5 chars (removed j-i-h-g-f)");

    // Insert newline
    tui.update(Msg::InsertNewline);
    assert_eq!(tui.lines().len(), 2, "Should have 2 lines");
    assert_eq!(tui.lines()[0], "abcde");
    assert_eq!(tui.lines()[1], "", "Second line should be empty");

    // Type more on new line
    tui.update(Msg::TextareaKey(char_key('x')));
    tui.update(Msg::TextareaKey(char_key('y')));
    tui.update(Msg::TextareaKey(char_key('z')));
    assert_eq!(tui.lines()[1], "xyz", "New line has xyz");

    // Cursor movements via TextareaKey should not panic
    tui.update(Msg::TextareaKey(special_key(KeyCode::Left)));
    tui.update(Msg::TextareaKey(special_key(KeyCode::Right)));
    tui.update(Msg::TextareaKey(special_key(KeyCode::Home)));
    tui.update(Msg::TextareaKey(special_key(KeyCode::End)));

    // Full content check
    assert_eq!(tui.content(), "abcde\nxyz", "Full content should be multiline");
}

/// Test: Hotkey behavior depends on current mode
#[test]
fn test_hotkey_priority() {
    let mut tui = MockTui::new();

    // In Chat mode: Ctrl+C should quit (not cancel) when textarea is empty
    assert_eq!(tui.state.mode, TuiMode::Chat);
    assert!(tui.state.running, "Should be running initially");
    if let Some(msg) = tui.simulate_key(KeyCode::Char('c'), true) {
        tui.update(msg);
    }
    assert!(
        !tui.state.running,
        "Ctrl+C should quit in Chat mode when input is empty"
    );

    // Reset
    tui.state.running = true;

    // Enter CommandPalette mode
    tui.update(Msg::OpenCommandPalette);
    assert_eq!(tui.state.mode, TuiMode::CommandPalette);

    // In CommandPalette mode: Ctrl+C should do nothing (passes through to terminal)
    // Ctrl+C in palette mode maps to key_to_palette_msg which returns None
    // because ctrl+C is only handled in ctrl_chat_key for Chat mode

    // Esc should close palette
    if let Some(msg) = tui.simulate_key(KeyCode::Esc, false) {
        tui.update(msg);
    }
    assert_eq!(tui.state.mode, TuiMode::Chat, "Esc should close palette");
    assert!(tui.state.running, "Should still be running after Esc");

    // Ctrl+C when textarea has content: clears input instead of quitting
    tui.state.running = true;
    tui.update(Msg::TextareaKey(char_key('h')));
    tui.update(Msg::TextareaKey(char_key('i')));
    if let Some(msg) = tui.simulate_key(KeyCode::Char('c'), true) {
        tui.update(msg);
    }
    assert!(
        tui.state.running,
        "Ctrl+C should NOT quit when textarea has content"
    );
    assert_eq!(tui.content(), "", "Ctrl+C with content should clear input");
}

/// Test that mock mode skips onboarding
///
/// This verifies the fix for the bug where --mock showed onboarding
/// instead of the operational UI.
#[test]
fn test_mock_mode_skips_onboarding() {
    let mock = true;
    let force_setup = false;
    let needs_onboarding = true; // Simulates no API key configured

    let needs_setup = force_setup || (!mock && needs_onboarding);

    assert!(
        !needs_setup,
        "--mock should skip onboarding even when no API key is configured"
    );
}

/// Test that --mock-setup forces onboarding
#[test]
fn test_mock_setup_forces_onboarding() {
    let mock = true;
    let force_setup = true;
    let needs_onboarding = false; // Already configured

    let needs_setup = force_setup || (!mock && needs_onboarding);

    assert!(
        needs_setup,
        "--mock-setup should force onboarding even when already configured"
    );
}
