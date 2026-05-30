use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::AgentEvent;

fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn })
        .handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", response), turn })
        .handle_agent_event(super::helpers::turn_end_event(turn))
}

/// Tests: user to agent response flow
#[test]
fn test_user_submit_message() {
    let harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");
    harness.assert_agent_not_running();
    harness.assert_has_user_message("Hello");
}

#[test]
fn test_agent_streaming_incremental() {
    let harness = AgentTestHarness::new();

    let harness = harness.submit_user_message("Hello");
    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    harness.assert_agent_running();
    harness.assert_has_assistant_placeholder();

    let harness = harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: super::helpers::agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });
    harness.assert_last_assistant_text("Hi");

    let harness = harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: super::helpers::agent_message("assistant", "Hi there"),
        turn: 1,
        delta: " there".to_string(),
    });
    harness.assert_last_assistant_text("Hi there");

    let harness = harness.handle_agent_event(AgentEvent::MessageEnd {
        message: super::helpers::agent_message("assistant", "Hi there!"),
        turn: 1,
    });

    harness.assert_last_assistant_text("Hi there!");
    assert!(!harness.state.is_thinking, "thinking should be false after MessageEnd");
    assert!(harness.state.status_header.is_none(), "status_header should be cleared after MessageEnd");
}

#[test]
fn test_multi_turn_conversation() {
    let harness = AgentTestHarness::new();

    // Turn 1
    let harness = harness.submit_user_message("Hello");
    let harness = complete_turn(harness, 1, "Hi there!");

    // Turn 2
    let harness = harness.submit_user_message("How are you?");
    let harness = complete_turn(harness, 2, "I'm doing well!");

    let separators: Vec<_> = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Separator { .. })).collect();
    assert_eq!(separators.len(), 2, "should have exactly 2 turn separators");

    let user_messages: Vec<_> = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::User { .. })).collect();
    assert_eq!(user_messages.len(), 2, "should have exactly 2 user messages");
}

#[test]
fn test_message_start_creates_placeholder_once() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    let assistant_count_after_first = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Assistant { .. })).count();
    assert_eq!(assistant_count_after_first, 1, "should have exactly 1 assistant message");

    // Second MessageStart should NOT add another
    let harness = harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    let assistant_count_after_second = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Assistant { .. })).count();
    assert_eq!(assistant_count_after_second, 1, "second MessageStart should NOT add another placeholder");
}

#[test]
fn test_turn_end_separator_requires_agent_start_time() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness
        .handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 })
        .handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", "Response"), turn: 1 });

    let separators_before: Vec<_> = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Separator { .. })).collect();
    assert!(separators_before.is_empty(), "no separator without agent_start_time");

    // Set agent_start_time and trigger turn end
    let mut harness = harness;
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    let harness = harness.handle_agent_event(super::helpers::turn_end_event(1));

    let separators_after: Vec<_> = harness.state.messages.iter().filter(|m| matches!(m, MessageItem::Separator { .. })).collect();
    assert_eq!(separators_after.len(), 1, "should have 1 separator after TurnEnd with agent_start_time");
}
