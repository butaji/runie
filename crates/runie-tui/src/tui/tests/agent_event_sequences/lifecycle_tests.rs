use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, TokenUsage};
use std::time::{Duration, Instant};

#[test]
fn test_agent_end_clears_agent_running() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    let harness = harness.handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", "Done"), turn: 1 });

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

#[test]
fn test_thinking_indicator_added_for_long_think() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    // Simulate some thinking time by directly setting thinking_start in the past
    harness.state.thinking_start = Some(Instant::now() - Duration::from_millis(1500));

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: super::helpers::agent_message("assistant", "Quick response"),
        turn: 1,
    });

    // Check that a Thought item was added (duration > 0.5s)
    let has_thought = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Thought { .. }));
    assert!(
        has_thought,
        "should have Thought item when thinking duration > 0.5s"
    );
}

#[test]
fn test_quick_think_no_indicator() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    // No delay - instant response

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: super::helpers::agent_message("assistant", "Hi"),
        turn: 1,
    });

    // Check that NO Thought item was added (duration < 0.5s)
    let has_thought = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Thought { .. }));
    assert!(
        !has_thought,
        "should NOT have Thought item when thinking duration < 0.5s"
    );
}
