//! Comprehensive test suite - Section 7: Error Recovery Tests (codex + pi).

use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart};

use super::harness::AgentTestHarness;
use super::state_tests::make_message;

#[test]
fn test_error_recovery() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::MessageEnd {
        message: make_message("assistant", "Done"),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    // Assert cleaned up
    assert!(!harness.state.agent_running);
    assert!(harness.state.status_header.is_none());

    // Can start new conversation
    harness = harness.user_says("Try again");
    assert!(!harness.state.agent_running); // Still not running until agent starts
}

#[test]
fn test_recoverable_error_flag() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Error {
            message: "timeout".to_string(),
            error_type: "timeout".to_string(),
            recoverable: true,
            context: "".to_string(),
        });

    let error_item = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(recoverable),
        _ => None,
    });

    assert_eq!(error_item, Some(&true));
}

#[test]
fn test_non_recoverable_error_flag() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Error {
            message: "panic".to_string(),
            error_type: "panic".to_string(),
            recoverable: false,
            context: "".to_string(),
        });

    let error_item = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(recoverable),
        _ => None,
    });

    assert_eq!(error_item, Some(&false));
}

#[test]
fn test_error_clears_thinking() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    assert!(harness.state.is_thinking);

    harness = harness.handle_event(AgentEvent::Error {
        message: "fail".to_string(),
        error_type: "test".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    assert!(!harness.state.is_thinking);
}

#[test]
fn test_error_adds_error_message() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Error {
            message: "Something went wrong".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        });

    assert!(harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Error { .. })));
}

#[test]
fn test_error_message_not_empty() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Error {
            message: "Error: connection refused".to_string(),
            error_type: "network".to_string(),
            recoverable: true,
            context: "".to_string(),
        });

    let error_text = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::Error { message, .. } => Some(message.as_str()),
        _ => None,
    });

    assert!(error_text.is_some());
    assert!(!error_text.unwrap().is_empty());
}

#[test]
fn test_multiple_errors() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Error {
            message: "error 1".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        })
        .handle_event(AgentEvent::Error {
            message: "error 2".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        });

    let error_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Error { .. }))
        .count();

    assert_eq!(error_count, 2);
}

#[test]
fn test_error_after_tool() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run tool then fail");

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        result: runie_agent::ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "files".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::Error {
        message: "failed after tool".to_string(),
        error_type: "test".to_string(),
        recoverable: false,
        context: "".to_string(),
    });

    // Tool should still be in messages
    assert!(harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::ToolCall { .. })));

    // Error should also be present
    assert!(harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Error { .. })));
}

#[test]
fn test_agent_end_after_message_start() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        })
        .handle_event(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        });

    harness.assert_agent_not_running();
}

#[test]
fn test_message_start_not_duplicated() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    let first_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();

    // Second MessageStart should NOT add another placeholder
    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    let second_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();

    assert_eq!(first_count, second_count);
}

#[test]
fn test_system_message_filtering() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Message {
            role: "system".to_string(),
            content: "Using gpt-4 model".to_string(),
        })
        .handle_event(AgentEvent::Message {
            role: "system".to_string(),
            content: "Mock mode enabled".to_string(),
        })
        .handle_event(AgentEvent::Message {
            role: "system".to_string(),
            content: "User authenticated".to_string(),
        });

    let system_messages: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::System { .. }))
        .collect();

    // Only "User authenticated" should pass through (doesn't start with "Using " or "Mock mode")
    assert_eq!(system_messages.len(), 1);
}

#[test]
fn test_empty_submission() {
    let mut state = crate::tui::state::AppState::default();
    state.current_model = Some("test-model".to_string());

    // Empty textarea
    let cmds = crate::tui::update::misc::handle_submit(&mut state);

    assert!(cmds.is_empty());
    assert!(state.messages.is_empty());
    assert!(state.input_right_info.contains("Type a message"));
}

#[test]
fn test_submit_blocked_when_agent_running() {
    let mut state = crate::tui::state::AppState::default();
    state.current_model = Some("test-model".to_string());
    state.agent_running = true;

    state.textarea.insert_str("Hello");
    let cmds = crate::tui::update::misc::handle_submit(&mut state);

    assert!(cmds.is_empty());
    assert!(state.messages.is_empty());
    assert!(state.input_right_info.contains("Agent running"));
}
