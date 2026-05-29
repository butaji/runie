use super::*;

#[test]
fn test_e2e_tool_error_displayed() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.agent_running = true;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "invalid command"}"#.to_string(),
        turn: 1,
    }));

    let error_result = runie_agent::events::ToolResult {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({"command": "invalid command"}),
        content: vec![ContentPart::Text { text: "command not found".to_string() }],
        is_error: true,
    };
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "invalid command"}"#.to_string(),
        result: error_result,
        duration_ms: 100,
        turn: 1,
    }));

    if let Some(MessageItem::ToolCall { is_error, result, .. }) = state.messages.last() {
        assert!(*is_error, "ToolCall is_error should be true");
        assert!(result.as_ref().unwrap().contains("command not found"));
    } else {
        panic!("Expected ToolCall message");
    }
}

#[test]
fn test_e2e_tool_not_found() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
        message: "Tool 'nonexistent_tool' not found".to_string(),
        error_type: "tool_not_found".to_string(),
        recoverable: true,
        context: "tool_execution".to_string(),
    }));

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { message, .. } if message.contains("nonexistent_tool"))));
    assert!(!state.agent_running);
}
