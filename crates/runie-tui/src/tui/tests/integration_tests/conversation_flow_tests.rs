use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::AgentEvent;

/// Helper: complete a turn with message start, update, end
fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart {
            message: super::helpers::agent_message("assistant", ""),
            turn,
        })
        .handle_agent_event(AgentEvent::MessageUpdate {
            message: super::helpers::agent_message("assistant", response),
            turn,
            delta: response.to_string(),
        })
        .handle_agent_event(AgentEvent::MessageEnd {
            message: super::helpers::agent_message("assistant", response),
            turn,
        })
        .handle_agent_event(AgentEvent::TurnEnd {
            turn,
            message_count: 2,
            tool_results_count: 0,
            token_usage: super::helpers::default_token_usage(),
        })
}

#[test]
fn test_first_turn_no_separator() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    assert_eq!(harness.state.messages.len(), 1, "user message only after submit");

    harness = complete_turn(harness, 1, "Hi");

    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: super::helpers::default_token_usage(),
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
        final_token_usage: super::helpers::default_token_usage(),
    });

    harness.submit_user_message("How are you?");
    assert_eq!(harness.state.messages.len(), 3, "+ user message");

    harness = complete_turn(harness, 2, "I'm good");
    assert_eq!(harness.state.messages.len(), 4, "+ assistant message");
}

#[test]
fn test_conversation_message_sequence() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: super::helpers::default_token_usage(),
    });

    harness.submit_user_message("How are you?");
    harness = complete_turn(harness, 2, "I'm good");

    let roles: Vec<&str> = harness.state.messages.iter().map(|m| match m {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Separator { .. } => "separator",
        _ => "other",
    }).collect();

    assert_eq!(roles, vec!["user", "assistant", "user", "assistant"], "correct message sequence");
}

/// Helper: complete turn with agent_start_time set
fn complete_turn_with_timing(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart {
            message: super::helpers::agent_message("assistant", ""),
            turn,
        })
        .handle_agent_event(AgentEvent::MessageEnd {
            message: super::helpers::agent_message("assistant", response),
            turn,
        })
        .handle_agent_event(AgentEvent::TurnEnd {
            turn,
            message_count: 2,
            tool_results_count: 0,
            token_usage: super::helpers::default_token_usage(),
        })
}

#[test]
fn test_turn_adds_separator_when_agent_started() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("Hello");
    harness = complete_turn_with_timing(harness, 1, "Hi");

    let separators: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(separators.len(), 1, "should have 1 separator after first turn");
}

#[test]
fn test_second_turn_also_adds_separator() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("Hello");
    harness = complete_turn_with_timing(harness, 1, "Hi");

    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.submit_user_message("How are you?");
    harness = complete_turn_with_timing(harness, 2, "I'm good");

    let separators: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(separators.len(), 2, "should have 2 separators after two turns");
}
