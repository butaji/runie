use super::*;

#[test]
fn test_e2e_tool_call_success() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Tool execution starts
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "tool_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        turn: 1,
    }));

    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::ToolCall { name, .. } if name == "tool_abc123"));

    // Tool execution ends successfully
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "tool_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        result: make_tool_result("tool_abc123", "bash", "file1\nfile2", false),
        duration_ms: 150,
        turn: 1,
    }));

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::ToolCall { result, is_error, .. } = &state.messages[0] {
        assert!(result.is_some());
        assert!(!*is_error);
    } else {
        panic!("Expected ToolCall message");
    }
}

#[test]
fn test_e2e_tool_call_error() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Tool execution starts
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "tool_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "invalid_command".to_string(),
        turn: 1,
    }));

    // Tool execution ends with error
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "tool_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "invalid_command".to_string(),
        result: make_tool_result("tool_err", "bash", "command not found", true),
        duration_ms: 50,
        turn: 1,
    }));

    if let MessageItem::ToolCall { is_error, .. } = &state.messages[0] {
        assert!(*is_error);
    } else {
        panic!("Expected ToolCall message");
    }
}

#[test]
fn test_e2e_tool_call_permission() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Agent requests permission
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_perm".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "rm -rf /".to_string(),
        tool_description: "Remove files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));

    // Should be in permission mode
    assert_eq!(state.mode, TuiMode::Permission);
    assert!(state.permission_modal.tool.is_some());
    assert_eq!(state.permission_modal.tool.as_deref(), Some("bash"));

    // User confirms permission
    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Should send Allow permission
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));
    assert_eq!(state.mode, TuiMode::Chat);
}
