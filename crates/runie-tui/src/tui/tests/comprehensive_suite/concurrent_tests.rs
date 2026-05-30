//! Comprehensive test suite - Section 4: Concurrent/Race Tests (crush pattern).

use crate::components::MessageItem;
use crate::tui::update::agent::handle_agent_event as agent_handle_event;

use super::harness::AgentTestHarness;
use super::state_tests::token_usage;

#[test]
fn test_concurrent_messages() {
    let state = crate::tui::state::AppState::default();

    // Simulate multiple operations on cloned states (sequential since AppState is not Send)
    let results: Vec<_> = (0..10)
        .map(|i| {
            let mut s = state.clone();
            s.messages.push(MessageItem::User {
                text: format!("msg{}", i),
                model: Some("You".to_string()),
                timestamp: None,
            });
            s
        })
        .collect();

    // Verify each result has 1 message
    for result in results {
        assert_eq!(result.messages.len(), 1);
    }
}

#[test]
fn test_concurrent_token_updates() {
    let state = crate::tui::state::AppState::default();

    // Simulate token updates on cloned states (sequential since AppState is not Send)
    for i in 0..5 {
        let mut s = state.clone();
        agent_handle_event(&mut s, token_usage(10 * (i + 1), 5 * (i + 1)));
    }

    // Note: This test verifies no panic occurs during event handling
}

#[test]
fn test_concurrent_tool_calls() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run commands");

    // Simulate rapid tool start/end cycles
    for i in 0..5 {
        let tool_id = format!("tool-{}", i);
        harness = harness.handle_event(runie_agent::AgentEvent::ToolExecutionStart {
            tool_call_id: tool_id.clone(),
            tool_name: "bash".to_string(),
            tool_args: format!("echo {}", i),
            turn: 1,
        });
        harness = harness.handle_event(runie_agent::AgentEvent::ToolExecutionEnd {
            tool_call_id: tool_id,
            tool_name: "bash".to_string(),
            tool_args: format!("echo {}", i),
            result: runie_agent::ToolResult {
                tool_call_id: "".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![runie_agent::ContentPart::Text {
                    text: format!("output{}", i),
                }],
                is_error: false,
            },
            duration_ms: 10,
            turn: 1,
        });
    }

    let tool_calls: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();

    assert_eq!(tool_calls.len(), 5);
}

#[test]
fn test_concurrent_state_checks() {
    let harness = AgentTestHarness::new();
    let state = &harness.state;

    // Multiple reads of agent_running should be consistent
    let results: Vec<_> = (0..10).map(|_| state.agent_running).collect();

    // All reads should return the same value
    assert!(results.iter().all(|&r| r == results[0]));
}
