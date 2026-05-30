use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use crate::tui::state::TuiMode;
use runie_agent::{AgentEvent, TokenUsage};
use std::time::Instant;

fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn })
        .handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", response), turn })
        .handle_agent_event(AgentEvent::TurnEnd { turn, message_count: 2, tool_results_count: 0, token_usage: super::helpers::default_token_usage() })
}

#[test]
fn test_first_turn_succeeds() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    assert!(harness.state.agent_running, "agent_running stays true after TurnEnd");
}

#[test]
fn test_second_turn_error() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");

    harness.submit_user_message("Cause error");
    harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 2,
    });
    harness = harness.handle_agent_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    assert!(!harness.state.agent_running, "agent should not be running after error");
    assert!(harness.state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })), "should have an error message");
    assert!(matches!(harness.state.mode, TuiMode::Chat), "should be in Chat mode after error");
}

#[test]
fn test_non_recoverable_error() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");
    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });
    let harness = harness.handle_agent_event(AgentEvent::Error {
        message: "Internal panic".to_string(),
        error_type: "panic".to_string(),
        recoverable: false,
        context: "".to_string(),
    });

    let error_item = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(recoverable),
        _ => None,
    });
    assert_eq!(error_item, Some(&false), "error should be marked as non-recoverable");
}

#[test]
fn test_tool_pauses_thinking() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("List files");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    assert!(harness.state.is_thinking, "should be thinking after MessageStart");

    let harness = harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    assert!(!harness.state.is_thinking, "should NOT be thinking after ToolExecutionStart");
    assert!(harness.state.thinking_duration.is_some(), "thinking_duration should be recorded");
}

#[test]
fn test_agent_end_clears_agent_running() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");
    let harness = harness
        .handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 })
        .handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", "Done"), turn: 1 });

    let mut harness = harness;
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.state.status_header = Some("Thinking".to_string());
    harness.state.is_thinking = true;

    let harness = harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: TokenUsage { input: 100, output: 50, cache_read: 0, cache_write: 0, total_tokens: 150 },
    });

    assert!(!harness.state.agent_running, "agent_running should be false after AgentEnd");
    assert!(harness.state.agent_start_time.is_none(), "agent_start_time should be None");
    assert!(!harness.state.is_thinking, "is_thinking should be false");
    assert!(harness.state.status_header.is_none(), "status_header should be None");
}
