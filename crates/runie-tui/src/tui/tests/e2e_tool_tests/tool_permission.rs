use super::*;

#[test]
fn test_e2e_tool_permission_allow() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        tool_description: "Execute bash command".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::Permission);
    assert_eq!(state.permission_modal.tool, Some("bash".to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::Allow { .. } }]));
    assert_eq!(state.mode, TuiMode::Chat);

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 1,
    }));

    let result = runie_agent::events::ToolResult {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({"command": "ls"}),
        content: vec![ContentPart::Text { text: "ls output".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result,
        duration_ms: 50,
        turn: 1,
    }));

    assert!(state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result: Some(r), .. } = m {
            r.contains("ls output")
        } else { false }
    }));
}

#[test]
fn test_e2e_tool_permission_deny() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_deny".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "rm -rf /"}"#.to_string(),
        tool_description: "Execute dangerous command".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    assert_eq!(cmds.len(), 2);
    assert!(matches!(&cmds[0], Cmd::SendPermission { decision: PermissionDecision::Deny { .. } }));
    assert!(matches!(&cmds[1], Cmd::Rollback { tool_call_id } if tool_call_id == "call_deny"));

    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_tool_permission_allow_always() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_first".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        tool_description: "Execute bash command".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);

    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } }]));
    assert_eq!(state.mode, TuiMode::Chat);

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_first".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 1,
    }));

    let result = runie_agent::events::ToolResult {
        tool_call_id: "call_first".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({"command": "ls"}),
        content: vec![ContentPart::Text { text: "result1".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_first".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result,
        duration_ms: 50,
        turn: 1,
    }));

    assert!(state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result: Some(r), .. } = m {
            r.contains("result1")
        } else { false }
    }));
}

#[test]
fn test_e2e_permission_queue_multiple() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        tool_description: "Execute bash".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "file.txt"}"#.to_string(),
        tool_description: "Read file".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.permission_modal.pending_queue.len(), 1);
    assert_eq!(state.permission_modal.pending_queue[0].tool_call_id, "call_2");

    assert_eq!(state.permission_modal.tool, Some("bash".to_string()));
    assert_eq!(state.permission_modal.tool_call_id, Some("call_1".to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::Allow { .. } }]));

    assert_eq!(state.permission_modal.tool, Some("read_file".to_string()));
    assert_eq!(state.permission_modal.tool_call_id, Some("call_2".to_string()));
    assert!(state.permission_modal.pending_queue.is_empty());
}
