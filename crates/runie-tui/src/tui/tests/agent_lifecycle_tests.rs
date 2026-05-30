//! Lifecycle and state management agent event sequence tests.

use super::agent_event_helpers::{agent_message, turn_end_event};
use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use crate::tui::state::Cmd;
use crate::tui::update::system::check_agent_timeout;
use runie_agent::{AgentEvent, TokenUsage};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn find_error_recoverable(harness: &AgentTestHarness) -> Option<bool> {
    harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    })
}

fn count_separators(harness: &AgentTestHarness) -> usize {
    harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Separator { .. })).count()
}

fn count_assistants(harness: &AgentTestHarness) -> usize {
    harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Assistant { .. })).count()
}

fn has_thought_message(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. }))
}

fn has_timeout_message(harness: &AgentTestHarness) -> bool {
    harness.state.messages.iter().any(|m| match m {
        MessageItem::System { text } => text.contains("timed out"),
        _ => false,
    })
}

fn make_token_usage() -> TokenUsage {
    TokenUsage {
        input: 100,
        output: 50,
        cache_read: 0,
        cache_write: 0,
        total_tokens: 150,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_non_recoverable_error() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::Error {
        message: "Internal panic".to_string(),
        error_type: "panic".to_string(),
        recoverable: false,
        context: "".to_string(),
    });

    assert_eq!(find_error_recoverable(&harness), Some(false), "error should be marked as non-recoverable");
}

#[test]
fn test_timeout_handling() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(601));
    let cmds = check_agent_timeout(&mut harness.state);

    assert!(!harness.state.agent_running, "agent_running should be false after timeout");
    assert!(harness.state.agent_start_time.is_none(), "agent_start_time should be cleared");
    assert!(!harness.state.is_thinking, "is_thinking should be false after timeout");
    assert!(cmds.is_some() && cmds.unwrap().contains(&Cmd::Interrupt), "should return Cmd::Interrupt");
    assert!(has_timeout_message(&harness), "should have a system message about timeout");
}

#[test]
fn test_agent_end_cleans_up_state() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Done"),
        turn: 1,
    });

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.state.status_header = Some("Thinking".to_string());
    harness.state.is_thinking = true;

    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: make_token_usage(),
    });

    assert!(!harness.state.agent_running, "agent_running should be false after AgentEnd");
    assert!(harness.state.agent_start_time.is_none(), "agent_start_time should be None");
    assert!(!harness.state.is_thinking, "is_thinking should be false after AgentEnd");
    assert!(harness.state.status_header.is_none(), "status_header should be None after AgentEnd");
}

#[test]
fn test_message_start_creates_placeholder_once() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    let first_count = count_assistants(&harness);
    assert_eq!(first_count, 1, "should have exactly 1 assistant message after first MessageStart");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    let second_count = count_assistants(&harness);
    assert_eq!(second_count, 1, "second MessageStart should NOT add another placeholder");
}

#[test]
fn test_turn_end_adds_separator() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Response"),
        turn: 1,
    });

    assert_eq!(count_separators(&harness), 0, "no separator without agent_start_time");

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    harness.handle_agent_event(turn_end_event(1));

    assert_eq!(count_separators(&harness), 1, "should have 1 separator after TurnEnd");
}

#[test]
fn test_thinking_indicator_added_for_long_think() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.state.thinking_start = Some(Instant::now() - Duration::from_millis(1500));

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Quick response"),
        turn: 1,
    });

    assert!(has_thought_message(&harness), "should have Thought item when thinking duration > 0.5s");
}

#[test]
fn test_quick_think_no_indicator() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });

    assert!(!has_thought_message(&harness), "should NOT have Thought item when thinking duration < 0.5s");
}

#[test]
fn test_tool_pauses_thinking() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("List files");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    assert!(harness.state.is_thinking, "should be thinking after MessageStart");

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    assert!(!harness.state.is_thinking, "should NOT be thinking after ToolExecutionStart");
    assert!(harness.state.thinking_duration.is_some(), "thinking_duration should be recorded");
}
