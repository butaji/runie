//! Basic agent event sequence tests.

use super::agent_event_helpers::{agent_message, turn_end_event, tool_result};
use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::AgentEvent;

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn submit_and_respond(harness: AgentTestHarness, user_text: &str, response: &str) -> AgentTestHarness {
    let mut h = harness.submit_user_message(user_text);
    h = h.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    h = h.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", response),
        turn: 1,
        delta: response.to_string(),
    });
    h.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", response),
        turn: 1,
    })
}

fn collect_messages_of_type(harness: &AgentTestHarness, variant: &str) -> Vec<&MessageItem> {
    harness.state.messages.iter().filter(|m| match (m, variant) {
        (MessageItem::User { .. }, "user") => true,
        (MessageItem::Separator { .. }, "separator") => true,
        _ => false,
    }).collect()
}

fn find_tool_result(harness: &AgentTestHarness, name: &str) -> Option<String> {
    harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall {
            name: n,
            result: Some(res),
            ..
        } if n == name => Some(res.clone()),
        _ => None,
    })
}

fn has_error_message(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. }))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_happy_path_user_to_agent_response() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.assert_agent_not_running();
    harness.assert_has_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();
    harness.assert_has_assistant_placeholder();

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });
    harness.assert_last_assistant_text("Hi");

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi there"),
        turn: 1,
        delta: " there".to_string(),
    });
    harness.assert_last_assistant_text("Hi there");

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi there!"),
        turn: 1,
    });

    harness.assert_last_assistant_text("Hi there!");
    assert!(!harness.state.is_thinking, "thinking should be false after MessageEnd");
    assert!(harness.state.status_header.is_none(), "status_header should be cleared after MessageEnd");
}

#[test]
fn test_tool_start_adds_tool_call() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
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
        assert!(!*is_error, "is_error should be false before ToolExecutionEnd");
    }
}

#[test]
fn test_tool_end_updates_result() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
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
        result: tool_result("total 42\ndrwxr-xr-x  5 admin admin  160 May 29 10:00 .", false),
        duration_ms: 150,
        turn: 1,
    });

    let result = find_tool_result(&harness, "call-1");
    assert!(result.is_some(), "tool call should have result after ToolExecutionEnd");
    assert!(result.unwrap().contains("total 42"), "tool result should contain expected output");
}

#[test]
fn test_tool_execution_with_error() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Run command");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        result: tool_result("Error: command failed with exit code 1", true),
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
fn test_multi_turn_conversation() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Hi there!"), turn: 1 });
    harness.handle_agent_event(turn_end_event(1));

    harness.submit_user_message("How are you?");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 2 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "I'm doing well!"), turn: 2 });
    harness.handle_agent_event(turn_end_event(2));

    let separators = collect_messages_of_type(&harness, "separator");
    assert_eq!(separators.len(), 2, "should have exactly 2 turn separators");

    let user_messages = collect_messages_of_type(&harness, "user");
    assert_eq!(user_messages.len(), 2, "should have exactly 2 user messages");
}

#[test]
fn test_error_recovery() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    harness.handle_agent_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    assert!(!harness.state.agent_running, "agent_running should be false after error");
    assert!(harness.state.status_header.is_none(), "status_header should be None after error");
    assert!(!harness.state.is_thinking, "is_thinking should be false after error");
    assert!(has_error_message(&harness), "should have an Error message item");
}
