//! Integration tests for conversation flow scenarios.

use super::integration_helpers::{agent_message, default_token_usage};
use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, TokenUsage};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn collect_message_roles(harness: &AgentTestHarness) -> Vec<&str> {
    harness.state.messages.iter().map(|m| match m {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Separator { .. } => "separator",
        _ => "other",
    }).collect()
}

fn count_separators(harness: &AgentTestHarness) -> usize {
    harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Separator { .. })).count()
}

fn make_token_usage(input: usize, output: usize) -> TokenUsage {
    TokenUsage {
        input,
        output,
        cache_read: 0,
        cache_write: 0,
        total_tokens: input + output,
    }
}

fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn })
        .handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", response), turn })
        .handle_agent_event(AgentEvent::TurnEnd { turn, message_count: 2, tool_results_count: 0, token_usage: default_token_usage() })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_first_turn_no_separator() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");

    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: default_token_usage(),
    });

    assert_eq!(harness.state.messages.len(), 2, "no separator without agent_start_time");
    assert!(!harness.state.agent_running, "agent_running cleared after AgentEnd");
}

#[test]
fn test_second_turn_appends_messages() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: default_token_usage(),
    });

    harness.submit_user_message("How are you?");
    harness = complete_turn(harness, 2, "I'm good");

    assert_eq!(collect_message_roles(&harness), vec!["user", "assistant", "user", "assistant"]);
}

#[test]
fn test_full_conversation_with_separator() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    assert_eq!(count_separators(&harness), 1, "should have 1 separator");

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.submit_user_message("How are you?");
    harness = complete_turn(harness, 2, "I'm good");
    assert_eq!(count_separators(&harness), 2, "should have 2 separators");
}

#[test]
fn test_permission_flow() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Run dangerous command");
    harness.handle_agent_event(AgentEvent::PermissionRequest {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "rm -rf /".to_string(),
        tool_description: "Delete everything".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    });

    assert!(matches!(harness.state.mode, crate::tui::state::TuiMode::Permission), "should be in Permission mode");
    assert!(harness.state.permission_modal.tool.is_some(), "permission modal should have tool set");
}

#[test]
fn test_token_usage_first_turn() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Hi"), turn: 1 });
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(10, 5) });

    assert_eq!(harness.state.session_token_usage.total_tokens, 15, "first turn tokens");
}

#[test]
fn test_token_usage_second_turn() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 1 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "Hi"), turn: 1 });
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(10, 5) });

    harness.submit_user_message("How are you?");
    harness.handle_agent_event(AgentEvent::MessageStart { message: agent_message("assistant", ""), turn: 2 });
    harness.handle_agent_event(AgentEvent::MessageEnd { message: agent_message("assistant", "I'm good"), turn: 2 });
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 15, completion_tokens: 10, total_tokens: 25, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 2, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(15, 10) });

    assert_eq!(harness.state.session_token_usage.total_tokens, 40, "total tokens should be 15 + 25");
    assert_eq!(harness.state.session_token_usage.prompt_tokens, 25, "prompt tokens: 10 + 15");
    assert_eq!(harness.state.session_token_usage.completion_tokens, 15, "completion tokens: 5 + 10");
}
