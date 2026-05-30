//! Comprehensive test suite - Section 7b: Stream Interruption and Edge Case Tests.

use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart};

use super::harness::AgentTestHarness;
use super::state_tests::make_message;

// ─── Stream Interruption Tests ─────────────────────────────────────────────

#[test]
fn test_stream_interruption_mid_message() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::MessageUpdate {
        message: make_message("assistant", "Partial"),
        turn: 1,
        delta: "Partial".to_string(),
    });

    // Simulate interruption (Ctrl+C)
    harness = harness.handle_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert!(!harness.state.agent_running);
    assert!(harness.state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { text, .. } if text == "Partial")));
}

#[test]
fn test_permission_timeout_event() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run tool");

    harness = harness.handle_event(AgentEvent::PermissionRequest {
        tool_call_id: "tool_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    });

    assert_eq!(harness.state.mode, crate::tui::state::TuiMode::Permission);
}

#[test]
fn test_multiple_permission_requests_queued() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run multiple tools");

    harness = harness.handle_event(AgentEvent::PermissionRequest {
        tool_call_id: "tool_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    });

    harness = harness.handle_event(AgentEvent::PermissionRequest {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "test.txt".to_string(),
        tool_description: "Read file".to_string(),
        turn: 1,
        context_window_usage: 0.1,
    });

    // Should queue the second request
    assert_eq!(harness.state.permission_modal.pending_queue.len(), 1);
}

#[test]
fn test_tool_error_result_displayed() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run failing tool");

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        result: runie_agent::ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "exit 1"}),
            content: vec![ContentPart::Text { text: "Error: exit code 1".to_string() }],
            is_error: true,
        },
        duration_ms: 50,
        turn: 1,
    });

    let tool_msg = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall { name, is_error, .. } if name == "t1" => Some(*is_error),
        _ => None,
    });
    assert_eq!(tool_msg, Some(true), "Tool error should be marked as error");
}

#[test]
fn test_message_end_without_updates() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::MessageEnd {
        message: make_message("assistant", ""),
        turn: 1,
    });

    let assistant_count = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 1, "Should have exactly one assistant message");
}

#[test]
fn test_turn_end_without_tools() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", "Hi"),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::MessageEnd {
        message: make_message("assistant", "Hi"),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: runie_agent::TokenUsage {
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 15,
        },
    });

    let separators = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .count();
    assert_eq!(separators, 1, "TurnEnd without tools should add separator");
}
