use super::*;

#[test]
fn test_e2e_permission_confirm() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_123".to_string());
    state.permission_modal.args = Some("ls".to_string());

    // Confirm permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Verify permission decision sent
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(state.permission_modal.tool.is_none());
}

#[test]
fn test_e2e_permission_deny() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_456".to_string());
    state.permission_modal.args = Some("rm -rf".to_string());

    // Deny permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    // Verify denial and rollback
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Deny { .. } })));
    assert!(cmds.iter().any(|c| matches!(c, Cmd::Rollback { tool_call_id } if tool_call_id == "tool_456")));
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_permission_always() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());
    state.permission_modal.args = Some("cat file".to_string());

    // Allow always
    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);

    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } })));
}

#[test]
fn test_e2e_permission_skip() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set up permission modal
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("read_file".to_string());
    state.permission_modal.tool_call_id = Some("tool_skip".to_string());
    state.permission_modal.args = Some("test.txt".to_string());

    // Skip permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);

    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Skip { .. } })));
    assert!(cmds.iter().any(|c| matches!(c, Cmd::Rollback { .. }))); // Skip triggers rollback
}

#[test]
fn test_e2e_permission_queue_in_blocking_mode() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Set blocking mode (Overlay)
    state.mode = TuiMode::Overlay;

    // Permission request while in blocking mode should be queued
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_queued".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    }));

    // Should queue the request instead of showing modal
    assert!(state.permission_modal.pending_queue.len() == 1);
    assert_eq!(state.mode, TuiMode::Overlay); // Mode unchanged

    // System message indicates queued
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("queued"))));
}
