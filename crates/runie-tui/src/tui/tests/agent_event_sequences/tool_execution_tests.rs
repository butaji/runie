use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::AgentEvent;

#[test]
fn test_tool_start_verification() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        turn: 1,
    });

    let tool_calls: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();
    assert_eq!(tool_calls.len(), 1, "should have exactly one tool call");

    if let MessageItem::ToolCall { name, args, result, is_error } = &tool_calls[0] {
        assert_eq!(name, "call-1");
        assert_eq!(args, "ls -la");
        assert!(result.is_none(), "result should be None before ToolExecutionEnd");
        assert!(!*is_error, "is_error should be false");
    }
}

#[test]
fn test_tool_end_updates_result() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        turn: 1,
    });

    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        result: super::helpers::tool_result("total 42\ndrwxr-xr-x  5 admin admin  160 May 29 10:00 .", false),
        duration_ms: 150,
        turn: 1,
    });

    let tool_result_item = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, result: Some(res), .. } if name == "call-1" => Some(res),
        _ => None,
    });
    assert!(tool_result_item.is_some(), "tool call should have result after ToolExecutionEnd");
    assert!(tool_result_item.unwrap().contains("total 42"), "tool result should contain expected output");
}

#[test]
fn test_tool_execution_with_error() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Run command");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        turn: 1,
    });

    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        result: super::helpers::tool_result("Error: command failed with exit code 1", true),
        duration_ms: 50,
        turn: 1,
    });

    let error_tool = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, is_error: true, .. } if name == "call_err" => Some(m),
        _ => None,
    });
    assert!(error_tool.is_some(), "should have an error tool call marked with is_error=true");
}

#[test]
fn test_tool_pauses_thinking() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    assert!(harness.state.is_thinking, "should be thinking after MessageStart");

    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    assert!(!harness.state.is_thinking, "should NOT be thinking after ToolExecutionStart");
    assert!(harness.state.thinking_duration.is_some(), "thinking_duration should be recorded");
}
