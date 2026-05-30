//! Integration tests for error handling scenarios.

use super::integration_helpers::agent_message;
use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, TokenUsage};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn make_token_usage() -> TokenUsage {
    TokenUsage {
        input: 100,
        output: 50,
        cache_read: 0,
        cache_write: 0,
        total_tokens: 150,
    }
}

fn has_error_message(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. }))
}

fn find_error_recoverable(harness: &AgentTestHarness) -> Option<bool> {
    harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_error_mid_conversation() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Hi"), turn: 1 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 0, token_usage: make_token_usage() });

    assert!(harness.state.agent_running, "agent_running stays true after TurnEnd");

    harness.submit_user_message("Cause error");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 2 });
    harness.handle_agent_event(AgentEvent::Error { message: "Network error".to_string(), error_type: "network".to_string(), recoverable: true, context: "".to_string() });

    assert!(!harness.state.agent_running, "agent should not be running after error");
    assert!(has_error_message(&harness), "should have an error message item");
    assert!(matches!(harness.state.mode, crate::tui::state::TuiMode::Chat), "should be in Chat mode after error");
}

#[test]
fn test_non_recoverable_error() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::Error { message: "Internal panic".to_string(), error_type: "panic".to_string(), recoverable: false, context: "".to_string() });

    assert_eq!(find_error_recoverable(&harness), Some(false), "error should be marked as non-recoverable");
}

#[test]
fn test_agent_end_cleans_up_state() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Done"), turn: 1 });

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.state.status_header = Some("Thinking".to_string());
    harness.state.is_thinking = true;

    harness.handle_agent_event(AgentEvent::AgentEnd { messages: vec![], total_turns: 1, final_token_usage: make_token_usage() });

    assert!(!harness.state.agent_running, "agent_running should be false after AgentEnd");
    assert!(harness.state.agent_start_time.is_none(), "agent_start_time should be None");
    assert!(!harness.state.is_thinking, "is_thinking should be false after AgentEnd");
    assert!(harness.state.status_header.is_none(), "status_header should be None after AgentEnd");
}
