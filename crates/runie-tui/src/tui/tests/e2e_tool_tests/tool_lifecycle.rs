use super::*;

#[test]
fn test_e2e_bash_tool_full_flow() {
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
    assert_eq!(state.permission_modal.tool_call_id, Some("call_1".to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);

    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 1,
    }));

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { name, .. } if name == "call_1")));

    let tool_result = runie_agent::events::ToolResult {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({"command": "ls"}),
        content: vec![ContentPart::Text { text: "file1.txt\nfile2.rs".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result: tool_result,
        duration_ms: 50,
        turn: 1,
    }));

    if let Some(MessageItem::ToolCall { result, is_error, .. }) = state.messages.last() {
        assert!(result.is_some(), "ToolCall should have result");
        assert!(!is_error, "ToolCall is_error should be false");
        assert!(result.as_ref().unwrap().contains("file1.txt"));
    } else {
        panic!("Expected ToolCall message");
    }
}

#[test]
fn test_e2e_read_file_tool_flow() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "src/main.rs"}"#.to_string(),
        tool_description: "Read file contents".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "src/main.rs"}"#.to_string(),
        turn: 1,
    }));

    let tool_result = runie_agent::events::ToolResult {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        input: serde_json::json!({"path": "src/main.rs"}),
        content: vec![ContentPart::Text { text: "fn main() {}".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "src/main.rs"}"#.to_string(),
        result: tool_result,
        duration_ms: 10,
        turn: 1,
    }));

    assert!(state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result: Some(r), .. } = m {
            r.contains("fn main()")
        } else {
            false
        }
    }));
}

#[test]
fn test_e2e_tool_chain_multiple() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_a".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 1,
    }));

    let result_a = runie_agent::events::ToolResult {
        tool_call_id: "call_a".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({"command": "ls"}),
        content: vec![ContentPart::Text { text: "output_a".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_a".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result: result_a,
        duration_ms: 50,
        turn: 1,
    }));

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_b".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "output_a"}"#.to_string(),
        turn: 1,
    }));

    let result_b = runie_agent::events::ToolResult {
        tool_call_id: "call_b".to_string(),
        tool_name: "read_file".to_string(),
        input: serde_json::json!({"path": "output_a"}),
        content: vec![ContentPart::Text { text: "content_of_file".to_string() }],
        is_error: false,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_b".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "output_a"}"#.to_string(),
        result: result_b,
        duration_ms: 30,
        turn: 1,
    }));

    let tool_calls: Vec<_> = state.messages.iter()
        .filter_map(|m| if let MessageItem::ToolCall { name, result, .. } = m {
            Some((name.clone(), result.clone()))
        } else { None })
        .collect();

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0].0, "call_a");
    assert!(tool_calls[0].1.as_ref().unwrap().contains("output_a"));
    assert_eq!(tool_calls[1].0, "call_b");
    assert!(tool_calls[1].1.as_ref().unwrap().contains("content_of_file"));
}
