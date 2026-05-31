//! Tests for Chat ↔ Overlay transitions.

use super::*;

/// Test: Chat → Overlay via SwitchModel.
#[test]
fn test_chat_to_overlay() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert_eq!(state.mode, TuiMode::Chat);

    // SwitchModel enters Overlay with model picker
    update(&mut state, &mut palette, Msg::SwitchModel);
    assert_eq!(state.mode, TuiMode::Overlay);
    assert!(state.model_picker.is_some());
}

/// Test: Overlay → Chat via CloseModal.
#[test]
fn test_overlay_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter overlay
    update(&mut state, &mut palette, Msg::SwitchModel);
    assert_eq!(state.mode, TuiMode::Overlay);

    // Close overlay
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(state.model_picker.is_none());
}

/// Test: Chat → Overlay → Chat round-trip.
#[test]
fn test_chat_overlay_chat_roundtrip() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Chat
    assert_eq!(state.mode, TuiMode::Chat);

    // To overlay
    update(&mut state, &mut palette, Msg::SwitchModel);
    assert_eq!(state.mode, TuiMode::Overlay);

    // Back to chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Esc closes overlay.
#[test]
fn test_esc_closes_overlay() {
    let state = make_state();
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::CloseModal));
}

/// Test: Ctrl+Q closes overlay (not quit).
#[test]
fn test_ctrl_q_closes_overlay() {
    let state = make_state();
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::CloseModal));
}

/// Test: Up/Down navigate overlay.
#[test]
fn test_up_down_navigate_overlay() {
    // Up
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectUp));

    // Down
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectDown));

    // Also test vim-style
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectUp));

    let msg = simulate_key(KeyCode::Char('j'), KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectDown));
}

/// Test: Enter confirms in overlay.
#[test]
fn test_enter_confirms_in_overlay() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectConfirm));
}

/// Test: Permission queued when in Overlay.
#[test]
fn test_permission_queued_in_overlay() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter overlay
    update(&mut state, &mut palette, Msg::SwitchModel);
    assert_eq!(state.mode, TuiMode::Overlay);

    // Permission request while in overlay should be queued
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_queued".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    }));

    // Should queue instead of showing modal
    assert!(state.permission_modal.pending_queue.len() == 1);
    assert_eq!(state.mode, TuiMode::Overlay); // Mode unchanged

    // System message indicates queued
    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::System { text } if text.contains("queued")
    )));
}

/// Test: Permission queued in DiffViewer.
#[test]
fn test_permission_queued_in_diff_viewer() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter DiffViewer mode
    state.mode = TuiMode::DiffViewer;
    state.diff_viewer = Some(crate::components::DiffViewer::new(
        "test.txt".to_string(),
        "old".to_string(),
        "new".to_string(),
    ));

    // Permission request should be queued
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_queued".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    }));

    assert!(state.permission_modal.pending_queue.len() == 1);
    assert_eq!(state.mode, TuiMode::DiffViewer); // Mode unchanged
}

/// Test: Permission queued in SessionTree.
#[test]
fn test_permission_queued_in_session_tree() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter SessionTree mode
    state.session_tree.toggle();
    state.mode = TuiMode::SessionTree;

    // Permission request should be queued
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_queued".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    }));

    assert!(state.permission_modal.pending_queue.len() == 1);
    assert_eq!(state.mode, TuiMode::SessionTree); // Mode unchanged
}
