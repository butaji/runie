//! Tests for Chat ↔ Permission transitions.

use super::*;

/// Test: Permission mode entered via AgentEvent.
#[test]
fn test_permission_mode_entered() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));

    assert_eq!(state.mode, TuiMode::Permission);
    assert!(state.permission_modal.tool.is_some());
}

/// Test: Permission → Chat via confirm.
#[test]
fn test_permission_confirm_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_123".to_string());

    // Confirm permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Verify permission decision sent
    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(state.permission_modal.tool.is_none());
}

/// Test: Permission → Chat via cancel.
#[test]
fn test_permission_cancel_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_456".to_string());

    // Cancel permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    // Verify denial and rollback
    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { decision: PermissionDecision::Deny { .. } })));
    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::Rollback { tool_call_id } if tool_call_id == "tool_456")));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Permission → Chat via PermissionAlways.
#[test]
fn test_permission_always_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());

    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);

    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } })));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Permission → Chat via PermissionSkip.
#[test]
fn test_permission_skip_to_chat() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("read_file".to_string());
    state.permission_modal.tool_call_id = Some("tool_skip".to_string());

    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);

    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { decision: PermissionDecision::Skip { .. } })));
    assert!(cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::Rollback { .. }))); // Skip triggers rollback
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Chat → Permission → Chat round-trip.
#[test]
fn test_chat_permission_chat_roundtrip() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Chat
    assert_eq!(state.mode, TuiMode::Chat);

    // To permission
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));
    assert_eq!(state.mode, TuiMode::Permission);

    // Back to chat
    update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Ctrl+C in Permission cancels (not quit).
#[test]
fn test_ctrl_c_cancels_permission() {
    let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel));
}

/// Test: Ctrl+Q in Permission cancels (not quit).
#[test]
fn test_ctrl_q_cancels_permission() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel));
}

/// Test: Enter confirms permission.
#[test]
fn test_enter_confirms_permission() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionConfirm));
}

/// Test: Esc cancels permission.
#[test]
fn test_esc_cancels_permission() {
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel));
}

/// Test: y/n keys for confirm/cancel.
#[test]
fn test_y_confirms_permission() {
    let msg = simulate_key(KeyCode::Char('y'), KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionConfirm));
}

#[test]
fn test_n_cancels_permission() {
    let msg = simulate_key(KeyCode::Char('n'), KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel));
}

/// Test: a for always.
#[test]
fn test_a_permission_always() {
    let msg = simulate_key(KeyCode::Char('a'), KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionAlways));
}

/// Test: s for skip.
#[test]
fn test_s_permission_skip() {
    let msg = simulate_key(KeyCode::Char('s'), KeyModifiers::NONE, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionSkip));
}

/// Test: AgentEnd during Permission clears modal.
#[test]
fn test_agent_end_clears_permission_modal() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());
    state.agent_running = true;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: AgentTokenUsage::default(),
    }));

    assert!(!state.agent_running, "agent_running should be cleared");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on AgentEnd");
    assert!(state.permission_modal.tool.is_none(), "Permission modal should be cleared");
}

/// Test: AgentEnd clears pending permission queue.
#[test]
fn test_agent_end_clears_pending_queue() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "-la".to_string(),
    });
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "file.txt".to_string(),
    });
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: AgentTokenUsage::default(),
    }));

    assert!(state.permission_modal.pending_queue.is_empty(),
        "AgentEnd should clear pending_queue");
    assert!(!state.agent_running);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test: Permission queue FIFO processing.
#[test]
fn test_permission_queue_fifo() {
    use crate::tui::state::PendingPermission;

    let mut queue: Vec<PendingPermission> = vec![
        PendingPermission {
            tool_call_id: "call_a".to_string(),
            tool_name: "A".to_string(),
            tool_args: "".to_string(),
        },
        PendingPermission {
            tool_call_id: "call_b".to_string(),
            tool_name: "B".to_string(),
            tool_args: "".to_string(),
        },
    ];

    let first = queue.remove(0);
    assert_eq!(first.tool_name.as_str(), "A", "remove(0) returns A first (FIFO)");

    let second = queue.remove(0);
    assert_eq!(second.tool_name.as_str(), "B", "Second remove(0) returns B");
}

/// Test: Multiple permissions queued, processed in order.
#[test]
fn test_multiple_permissions_queued_and_processed() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Set up first permission
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());

    // Queue another
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "file.txt".to_string(),
    });

    // Confirm first
    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Should have allowed first and still have second queued
    assert_eq!(state.mode, TuiMode::Permission); // Still in permission for second
    assert_eq!(state.permission_modal.pending_queue.len(), 0);
    assert!(state.permission_modal.tool.is_some()); // Second permission now active
}

/// Test: Permission always/once decision persisted in state.
#[test]
fn test_permission_decision_persists() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // This is a simplified test - actual persistence would need to check
    // some form of permanent storage which is outside AppState
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_decision".to_string());

    // Confirm with always
    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);

    // Verify Always decision was sent
    assert!(cmds.iter().any(|c| matches!(
        c,
        crate::tui::state::Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } }
    )));
}
