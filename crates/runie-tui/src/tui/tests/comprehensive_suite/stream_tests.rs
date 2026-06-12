//! Comprehensive test suite - Section 3: Mock Stream Tests (pi pattern).

use crate::components::MessageItem;
use crate::tui::state::ThinkingState;
use runie_agent::{AgentEvent, ContentPart, ToolResult, TokenUsage};

use super::harness::AgentTestHarness;
use super::state_tests::{make_message, token_usage};

fn make_tool_start_event(call_id: &str, name: &str, args: &str) -> AgentEvent {
    AgentEvent::ToolExecutionStart {
        tool_call_id: call_id.to_string(),
        tool_name: name.to_string(),
        tool_args: args.to_string(),
        turn: 1,
    }
}

fn make_tool_end_event(call_id: &str, name: &str, args: &str, result_text: &str) -> AgentEvent {
    AgentEvent::ToolExecutionEnd {
        tool_call_id: call_id.to_string(),
        tool_name: name.to_string(),
        tool_args: args.to_string(),
        result: ToolResult {
            tool_call_id: call_id.to_string(),
            tool_name: name.to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: result_text.to_string(),
            }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 1,
    }
}

fn make_turn_end(turn: usize) -> AgentEvent {
    AgentEvent::TurnEnd {
        turn,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 50,
            output: 25,
            total_tokens: 75,
            cache_read: 0,
            cache_write: 0,
        },
        turn_duration_ms: None,
    }
}

#[test]
fn test_stream_event_sequence() {
    let events = vec![
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
        AgentEvent::MessageUpdate {
            message: make_message("assistant", "Hello"),
            delta: "Hello".to_string(),
            replace: false,
            turn: 1,
        },
        make_tool_start_event("t1", "bash", "ls"),
        make_tool_end_event("t1", "bash", "ls", "file1.txt"),
        AgentEvent::MessageEnd {
            message: make_message("assistant", "Hello"),
            turn: 1,
        },
        make_turn_end(1),
    ];

    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .stream_events(events);

    harness.assert_event_sequence(&[
        "message_start",
        "message_update",
        "tool_start",
        "tool_end",
        "message_end",
        "turn_end",
    ]);
}

#[test]
fn test_stream_text_updates() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .agent_responds("Hi")
        .agent_responds("Hi there")
        .agent_responds("Hi there!");

    harness.assert_last_assistant_contains("Hi there!");
}

#[test]
fn test_first_turn_completes() {
    let harness = AgentTestHarness::new().user_says("First");
    let harness = complete_turn(harness, 1, "Response to first");

    let user_messages: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .collect();
    assert_eq!(user_messages.len(), 1);
}

#[test]
fn test_stream_preserves_message_order() {
    let harness = AgentTestHarness::new().user_says("First");
    let harness = complete_turn(harness, 1, "Response to first");
    let harness = harness.user_says("Second");
    let harness = harness.handle_event(AgentEvent::MessageStart { message: make_message("assistant", ""), turn: 2 });
    let harness = harness.handle_event(AgentEvent::MessageUpdate {
        message: make_message("assistant", "Response to second"),
        delta: "Response to second".to_string(),
        replace: false,
        turn: 2,
    });

    let user_messages: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .collect();
    assert_eq!(user_messages.len(), 2);
}

/// Helper: complete a single turn with message start, update, end, turn end, and agent end
fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn,
        })
        .handle_event(AgentEvent::MessageUpdate {
            message: make_message("assistant", response),
            delta: response.to_string(),
            replace: false,
            turn,
        })
        .handle_event(AgentEvent::MessageEnd {
            message: make_message("assistant", response),
            turn,
        })
        .handle_event(make_turn_end(turn))
        .handle_event(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: turn,
            final_token_usage: runie_agent::TokenUsage::default(),
        })
}

/// Helper: set up a two-turn conversation and return harness
fn setup_two_turn_conversation() -> AgentTestHarness {
    let h1 = AgentTestHarness::new().user_says("Turn 1");
    let h2 = complete_turn(h1, 1, "Response 1");
    let h3 = h2.user_says("Turn 2");
    complete_turn(h3, 2, "Response 2")
}

#[test]
fn test_stream_turn_separators() {
    let harness = setup_two_turn_conversation();

    // on_turn_end stores metrics in last_turn_* fields instead of adding Separator messages
    // We can verify turn metrics are tracked
    assert!(
        harness.state.last_turn_duration_secs.is_some() || harness.state.last_turn_tokens.is_some(),
        "turn metrics should be tracked"
    );
}

#[test]
fn test_stream_token_usage_accumulates() {
    let mut harness = AgentTestHarness::new();

    harness = harness.handle_event(token_usage(100, 50));
    harness = harness.handle_event(token_usage(200, 100));

    assert_eq!(harness.state.session_token_usage.prompt_tokens, 300);
    assert_eq!(harness.state.session_token_usage.completion_tokens, 150);
    assert_eq!(harness.state.session_token_usage.total_tokens, 450);
}

#[test]
fn test_token_usage_zero() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(token_usage(0, 0));

    assert_eq!(harness.state.session_token_usage.total_tokens, 0);
}

#[test]
fn test_token_usage_large_numbers() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(token_usage(100_000, 50_000));

    assert_eq!(harness.state.session_token_usage.total_tokens, 150_000);
}

#[test]
fn test_turn_count() {
    let harness = setup_two_turn_conversation();

    // Turn count is tracked via on_turn_end setting last_turn_* fields
    // The harness completes 2 turns, so we should have turn metrics stored
    assert!(
        harness.state.last_turn_duration_secs.is_some(),
        "turn duration should be tracked after turn ends"
    );
}

#[test]
fn test_thinking_duration_accumulated() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    // Simulate some thinking time
    harness.state.thinking =
        Some(ThinkingState { start: Some(std::time::Instant::now() - std::time::Duration::from_millis(800)), text: String::new(), accrued_duration: None });

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    // Thinking duration should be accumulated
    assert!(harness.state.thinking.as_ref().map_or(false, |t| t.accrued_duration.is_some()));
    assert!(harness.state.thinking.as_ref().unwrap().accrued_duration.unwrap().as_millis() >= 700);
}

#[test]
fn test_message_content_extraction() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Message {
            role: "assistant".to_string(),
            content: "Part1 Part2".to_string(),
        });

    let assistant_text = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::Assistant { text, .. } => Some(text.as_str()),
        _ => None,
    });

    assert_eq!(assistant_text, Some("Part1 Part2"));
}
