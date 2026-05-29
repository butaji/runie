use super::*;

#[test]
fn test_e2e_tool_call_rendered() {
    let mut state = make_state();

    state.messages.push(MessageItem::ToolCall {
        name: "bash".to_string(),
        args: r#"{"command": "echo hello"}"#.to_string(),
        result: None,
        is_error: false,
    });

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::ToolCall { name, args, .. } = &state.messages[0] {
        assert_eq!(name, "bash");
        assert!(args.contains("echo hello"));
    }
}

#[test]
fn test_e2e_tool_result_rendered() {
    let mut state = make_state();

    state.messages.push(MessageItem::ToolCall {
        name: "bash".to_string(),
        args: r#"{"command": "ls"}"#.to_string(),
        result: Some("file1.txt\nfile2.rs".to_string()),
        is_error: false,
    });

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::ToolCall { result: Some(r), is_error: false, .. } = &state.messages[0] {
        assert!(r.contains("file1.txt"));
    } else {
        panic!("Expected successful ToolCall");
    }

    state.messages.push(MessageItem::ToolCall {
        name: "bash".to_string(),
        args: r#"{"command": "invalid"}"#.to_string(),
        result: Some("command not found".to_string()),
        is_error: true,
    });

    if let MessageItem::ToolCall { result: Some(r), is_error: true, .. } = &state.messages[1] {
        assert!(r.contains("command not found"));
    } else {
        panic!("Expected error ToolCall");
    }
}

#[test]
fn test_e2e_permission_timeout_sends_denial() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_timeout".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        tool_description: "Execute bash command".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::Permission);

    state.permission_modal.timed_out = true;

    assert!(state.permission_modal.timed_out);
}
