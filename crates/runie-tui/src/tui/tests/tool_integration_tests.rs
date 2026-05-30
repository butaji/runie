//! Integration tests for tool execution scenarios.

use super::integration_helpers::{agent_message, default_token_usage};
use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart, ToolResult};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn make_tool_result(text: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text { text: text.to_string() }],
        is_error,
    }
}

fn has_tool_call(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. }))
}

fn has_separator(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| matches!(m, MessageItem::Separator { .. }))
}

fn count_tool_calls(harness: &AgentTestHarness) -> usize {
    harness.state.messages.iter().filter(|m| matches!(m, MessageItem::ToolCall { .. })).count()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_agent_with_tool_use() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    harness.submit_user_message("List files");

    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::ToolExecutionStart { tool_call_id: "t1".to_string(), tool_name: "bash".to_string(), tool_args: "ls".to_string(), turn: 1 });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd { tool_call_id: "t1".to_string(), tool_name: "bash".to_string(), tool_args: "ls".to_string(), result: make_tool_result("file1.txt\nfile2.rs", false), duration_ms: 50, turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageUpdate { message: agent_message("assistant", "Here are the files"), turn: 1, delta: "Here are the files".to_string() });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Here are the files"), turn: 1 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 1, token_usage: default_token_usage() });

    assert!(has_tool_call(&harness), "should have a tool call");
    assert!(has_separator(&harness), "should have a separator after turn");
}

#[test]
fn test_multiple_tools_in_one_turn() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    harness.submit_user_message("List files and check git status");

    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });

    harness.handle_agent_event(AgentEvent::ToolExecutionStart { tool_call_id: "t1".to_string(), tool_name: "bash".to_string(), tool_args: "ls".to_string(), turn: 1 });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd { tool_call_id: "t1".to_string(), tool_name: "bash".to_string(), tool_args: "ls".to_string(), result: make_tool_result("file1.txt\nfile2.rs", false), duration_ms: 50, turn: 1 });

    harness.handle_agent_event(AgentEvent::ToolExecutionStart { tool_call_id: "t2".to_string(), tool_name: "bash".to_string(), tool_args: "git status".to_string(), turn: 1 });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd { tool_call_id: "t2".to_string(), tool_name: "bash".to_string(), tool_args: "git status".to_string(), result: make_tool_result("On branch main", false), duration_ms: 100, turn: 1 });

    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "I found 2 files and you're on main branch"), turn: 1 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 2, token_usage: default_token_usage() });

    assert_eq!(count_tool_calls(&harness), 2, "should have exactly 2 tool calls");
}

#[test]
fn test_tool_pauses_thinking() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("List files");

    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    assert!(harness.state.is_thinking, "should be thinking after MessageStart");

    harness.handle_agent_event(AgentEvent::ToolExecutionStart { tool_call_id: "t1".to_string(), tool_name: "bash".to_string(), tool_args: "ls".to_string(), turn: 1 });

    assert!(!harness.state.is_thinking, "should NOT be thinking after ToolExecutionStart");
    assert!(harness.state.thinking_duration.is_some(), "thinking_duration should be recorded");
}
