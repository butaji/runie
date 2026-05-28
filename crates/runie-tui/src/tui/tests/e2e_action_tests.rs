//! End-to-end tests for user actions (keyboard shortcuts, commands).
//!
//! These tests verify that user actions produce correct state changes and commands.

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use crate::components::CommandPalette;
use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode};
use crate::tui::update::update;

/// Simulate a key press and return the resulting Msg.
fn simulate_key(code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers, state: &AppState) -> Option<Msg> {
    use crossterm::event::{Event, KeyEvent, KeyEventKind, KeyEventState};
    let event = Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    crate::tui::events::event_to_msg(event, state).into_iter().next()
}

/// Create default AppState in Chat mode.
fn make_state() -> AppState {
    AppState::default()
}

/// Create AppState with text in textarea.
fn make_state_with_text(text: &str) -> AppState {
    let mut state = AppState::default();
    state.textarea.insert_str(text);
    state
}

/// Create AppState with a specific model set.
fn make_state_with_model(model: &str) -> AppState {
    let mut state = AppState::default();
    state.current_model = Some(model.to_string());
    state
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEYBOARD SHORTCUTS
// ═══════════════════════════════════════════════════════════════════════════════

/// test_e2e_ctrl_c_clear_input:
///
/// 1. Type text in textarea
/// 2. Press Ctrl+C once → "Press again to clear" hint
/// 3. Press Ctrl+C twice within 2s → input cleared
#[test]
fn test_e2e_ctrl_c_clear_input() {
    let mut state = make_state_with_text("hello world");
    let mut palette = CommandPalette::new();

    // Initially textarea has content
    assert!(!state.textarea.is_empty());

    // First Ctrl+C → hint shown, not cleared
    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty(), "First tap should NOT clear");
    assert!(state.input_right_info.contains("Ctrl+C again"),
        "Should show hint: {}", state.input_right_info);

    // Second Ctrl+C within 2s → cleared
    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(state.textarea.is_empty(), "Second tap should clear");
}

/// test_e2e_ctrl_b_toggle_sidebar:
///
/// - show_sidebar=false
/// - Press Ctrl+B → show_sidebar=true
/// - Press Ctrl+B → show_sidebar=false
#[test]
fn test_e2e_ctrl_b_toggle_sidebar() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert!(!state.show_sidebar);

    // Ctrl+B → ToggleSidebar
    let msg = simulate_key(crossterm::event::KeyCode::Char('b'),
                          crossterm::event::KeyModifiers::CONTROL, &state);
    assert_eq!(msg, Some(Msg::ToggleSidebar));

    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(state.show_sidebar);

    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}

/// test_e2e_ctrl_p_open_palette:
///
/// - mode=Chat
/// - Press Ctrl+P → mode=CommandPalette
/// - command_palette.open=true
#[test]
fn test_e2e_ctrl_p_open_palette() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert_eq!(state.mode, TuiMode::Chat);

    // Ctrl+P → OpenCommandPalette
    let msg = simulate_key(crossterm::event::KeyCode::Char('p'),
                          crossterm::event::KeyModifiers::CONTROL, &state);
    assert_eq!(msg, Some(Msg::OpenCommandPalette));

    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);
    assert!(state.command_palette.open);
}

/// test_e2e_esc_close_modal:
///
/// - mode=CommandPalette
/// - Press Esc → mode=Chat
/// - mode=Permission
/// - Press Esc → mode=Chat (denies permission)
#[test]
fn test_e2e_esc_close_modal() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Test: Esc closes CommandPalette
    state.mode = TuiMode::CommandPalette;
    state.command_palette.open = true;

    let msg = simulate_key(crossterm::event::KeyCode::Esc,
                          crossterm::event::KeyModifiers::NONE, &state);
    assert_eq!(msg, Some(Msg::CommandPaletteCancelArgument));

    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
    assert_eq!(state.mode, TuiMode::Chat);

    // Test: Esc cancels permission
    state.mode = TuiMode::Permission;
    let msg = simulate_key(crossterm::event::KeyCode::Esc,
                          crossterm::event::KeyModifiers::NONE, &state);
    assert_eq!(msg, Some(Msg::PermissionCancel));

    update(&mut state, &mut palette, Msg::PermissionCancel);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// test_e2e_pgup_pgdown_scroll:
///
/// - Add 20 messages
/// - Press PgUp → scroll.feed_offset changes (decreases)
/// - Press PgDown → scroll.feed_offset changes back (increases)
#[test]
fn test_e2e_pgup_pgdown_scroll() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add 20 messages
    for i in 0..20 {
        state.messages.push(MessageItem::User {
            text: format!("message {}", i),
            model: Some("You".to_string()),
            timestamp: None,
        });
    }

    // Set initial offset to 10 so PgUp can decrease it
    state.scroll.feed_offset = 10;
    let initial_offset = state.scroll.feed_offset;

    // PgUp → ScrollPageUp
    let msg = simulate_key(crossterm::event::KeyCode::PageUp,
                          crossterm::event::KeyModifiers::NONE, &state);
    assert_eq!(msg, Some(Msg::ScrollPageUp));

    update(&mut state, &mut palette, Msg::ScrollPageUp);
    assert!(state.scroll.feed_offset < initial_offset,
        "PgUp should decrease offset from {}", initial_offset);

    let offset_after_pgup = state.scroll.feed_offset;

    // PgDown → ScrollPageDown
    let msg = simulate_key(crossterm::event::KeyCode::PageDown,
                          crossterm::event::KeyModifiers::NONE, &state);
    assert_eq!(msg, Some(Msg::ScrollPageDown));

    update(&mut state, &mut palette, Msg::ScrollPageDown);
    assert!(state.scroll.feed_offset > offset_after_pgup,
        "PgDown should increase offset from {}", offset_after_pgup);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SLASH COMMANDS
// ═══════════════════════════════════════════════════════════════════════════════

/// test_e2e_slash_clear:
///
/// - Type "/clear"
/// - Submit → messages cleared
#[test]
fn test_e2e_slash_clear() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add some messages
    state.messages.push(MessageItem::User {
        text: "hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    assert!(!state.messages.is_empty());

    // Simulate /clear command
    let cmd = runie_core::slash_command::SlashCommand::Clear;
    update(&mut state, &mut palette, Msg::SlashCommand(cmd));

    assert!(state.messages.is_empty(), "Messages should be cleared");
}

/// test_e2e_slash_model:
///
/// - Type "/model gpt-4o"
/// - Submit → model switched, confirmation message
#[test]
fn test_e2e_slash_model() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert!(state.current_model.is_none());

    // Simulate /model gpt-4o command
    let cmd = runie_core::slash_command::SlashCommand::Model("gpt-4o".to_string());
    update(&mut state, &mut palette, Msg::SlashCommand(cmd));

    assert_eq!(state.current_model, Some("gpt-4o".to_string()));
    assert!(!state.messages.is_empty());
    if let MessageItem::System { text } = &state.messages[0] {
        assert!(text.contains("gpt-4o"));
    }
}

/// test_e2e_slash_quit:
///
/// - Type "/quit"
/// - Submit → running=false
#[test]
fn test_e2e_slash_quit() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert!(state.running);

    // Simulate /quit command
    let cmd = runie_core::slash_command::SlashCommand::Quit;
    update(&mut state, &mut palette, Msg::SlashCommand(cmd));

    assert!(!state.running);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASTE
// ═══════════════════════════════════════════════════════════════════════════════

/// test_e2e_paste_in_chat:
///
/// - Paste "hello world"
/// - textarea contains "hello world"
#[test]
fn test_e2e_paste_in_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert!(state.textarea.is_empty());

    // Paste "hello world"
    update(&mut state, &mut palette, Msg::Paste("hello world".to_string()));

    let text = state.textarea.lines().join("");
    assert!(text.contains("hello world"), "Textarea should contain pasted text");
}

/// test_e2e_paste_blocked_in_permission:
///
/// - mode=Permission
/// - Paste "hello"
/// - textarea unchanged (paste ignored)
#[test]
fn test_e2e_paste_blocked_in_permission() {
    let state = make_state();
    let state = AppState {
        mode: TuiMode::Permission,
        ..state
    };

    // event_to_msg should return empty vec for paste in Permission mode
    let event = crossterm::event::Event::Paste("hello".to_string());
    let msgs = crate::tui::events::event_to_msg(event, &state);
    assert!(msgs.is_empty(), "Paste should be blocked in Permission mode");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUBMIT BLOCKING
// ═══════════════════════════════════════════════════════════════════════════════

/// test_e2e_submit_blocked_when_agent_running:
///
/// - agent_running=true
/// - Type text → Submit → blocked, no SpawnAgent
/// - input_right_info shows "Agent running..."
#[test]
fn test_e2e_submit_blocked_when_agent_running() {
    let mut state = make_state_with_model("gpt-4o");
    let mut palette = CommandPalette::new();

    state.agent_running = true;
    state.textarea.insert_str("hello");

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // No SpawnAgent cmd
    assert!(cmds.is_empty(), "Should not spawn agent when already running");
    // No message added
    assert!(state.messages.is_empty(), "No message should be added");
    // Feedback shown
    assert!(state.input_right_info.contains("running") ||
            state.input_right_info.contains("stop"),
        "Should show running feedback: {}", state.input_right_info);
}

/// test_e2e_submit_blocked_no_model:
///
/// - current_model=None
/// - Type text → Submit → System message added
/// - No SpawnAgent
#[test]
fn test_e2e_submit_blocked_no_model() {
    let mut state = make_state();
    state.current_model = None;
    state.onboarding = None;
    let mut palette = CommandPalette::new();

    state.textarea.insert_str("hello");

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // No SpawnAgent
    assert!(cmds.is_empty(), "Should not spawn agent without model");
    // System message about missing model
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { .. })),
        "Should add system message about missing model");
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOUBLE-TAP TIMEOUT
// ═══════════════════════════════════════════════════════════════════════════════

/// test_e2e_double_tap_timeout:
///
/// - Press Ctrl+C once → pending=true
/// - Wait 3 seconds
/// - Press Ctrl+C → treated as first tap again (timeout expired)
#[test]
fn test_e2e_double_tap_timeout() {
    let mut state = make_state_with_text("hello world");
    let mut palette = CommandPalette::new();

    // First tap
    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(state.clear_input_confirm.is_pending(),
        "Should be pending after first tap");
    assert!(!state.textarea.is_empty(),
        "Should NOT clear after first tap");

    // Simulate timeout (3 seconds elapsed)
    state.clear_input_confirm.last_press = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(3)
    );

    // Next tap should be treated as first tap (timeout expired)
    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty(),
        "After timeout, should NOT clear - treated as first tap");
    assert!(state.clear_input_confirm.is_pending(),
        "Should be pending again after timeout-reset tap");
}
