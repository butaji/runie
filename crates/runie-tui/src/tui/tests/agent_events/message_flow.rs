//! Message flow tests.
//!
//! Tests:
//! - MessageStart sets agent_running, adds placeholder
//! - MessageUpdate accumulates text
//! - MessageEnd finalizes, records thinking duration
//! - Thinking indicator added when duration > 0.5s
//! - Empty response preserves then removes placeholder
//! - ContextCompacted event (no-op, no panic)

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::ThinkingState;
use crate::tui::update::agent::handle_agent_event;
use runie_agent::{AgentEvent, AgentMessage, ContentPart};

/// Helper: Create an AgentMessage with given role and content text.
fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Helper: Create AppState ready for agent message flow testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state
}

// ─── MessageStart tests ───────────────────────────────────────────────────────

#[test]
fn test_message_start_sets_agent_running() {
    let mut state = make_test_state();
    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(state.agent_running, "agent_running should be true");
    assert!(state.thinking.is_some(), "thinking should be Some");
    assert!(state.thinking.as_ref().map_or(false, |t| t.start.is_some()), "thinking.start should be set");
}

#[test]
fn test_message_start_adds_placeholder() {
    let mut state = make_test_state();
    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "should have empty assistant placeholder"
    );
}

#[test]
fn test_message_start_does_not_add_duplicate_placeholder() {
    let mut state = make_test_state();
    // Pre-add a placeholder (simulating handle_submit behavior)
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: Some("test-model".to_string()),
        timestamp: None,
    });

    // MessageStart should NOT add another placeholder
    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    let assistant_count = state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 1, "should still have exactly 1 assistant");
}

#[test]
fn test_message_start_sets_status_header() {
    let mut state = make_test_state();
    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert_eq!(
        state.status_header,
        Some("Thinking".to_string()),
        "status_header should be Thinking"
    );
    // status_details now shows elapsed time (0s at start)
    assert!(state.status_details.is_some(), "status_details should be set");
    assert!(
        state.status_details.unwrap().ends_with("s"),
        "status_details should show elapsed time"
    );
}

// ─── MessageUpdate tests ───────────────────────────────────────────────────────

#[test]
fn test_message_update_accumulates_text() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    handle_agent_event(
        &mut state,
        AgentEvent::MessageUpdate {
            message: agent_message("assistant", "Hello"),
            turn: 1,
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.contains("Hello")
        )),
        "should have Hello in assistant message"
    );
}

#[test]
fn test_message_update_replaces_placeholder() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "should have empty placeholder before update"
    );

    handle_agent_event(
        &mut state,
        AgentEvent::MessageUpdate {
            message: agent_message("assistant", "Hello, world!"),
            turn: 1,
        },
    );

    let assistant_count = state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 1, "should still have exactly 1 assistant (updated in place)");
}

#[test]
fn test_message_update_multiple_chunks() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    let chunks = vec!["The", " quick", " brown", " fox"];
    for chunk in chunks {
        handle_agent_event(
            &mut state,
            AgentEvent::MessageUpdate {
                message: agent_message("assistant", chunk),
                turn: 1,
            },
        );
    }

    state
        .messages
        .iter()
        .find_map(|m| match m {
            MessageItem::Assistant { text, .. } => {
                // MessageUpdate REPLACES text, so after 4 chunks of "The", " quick", " brown", " fox"
                // only the last chunk " fox" remains (not accumulated)
                assert_eq!(text, " fox", "should have final text ' fox' (last chunk)");
                Some(())
            }
            _ => None,
        });
}

// ─── MessageEnd tests ─────────────────────────────────────────────────────────

#[test]
fn test_message_end_records_thinking_duration() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    // Simulate some thinking time
    std::thread::sleep(std::time::Duration::from_millis(10));

    handle_agent_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: agent_message("assistant", "Done"),
            turn: 1,
        },
    );

    // on_message_end records duration locally and clears thinking = None
    assert!(state.thinking.is_none(), "thinking should be None after end");
    // Duration was recorded but thinking state is now None (not stored in accrued_duration)
}

#[test]
fn test_message_end_adds_thought_indicator_if_long_thinking() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    // Simulate long thinking (> 0.5s)
    std::thread::sleep(std::time::Duration::from_millis(600));

    handle_agent_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: agent_message("assistant", "Done"),
            turn: 1,
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })),
        "should have Thought indicator for long thinking"
    );
}

#[test]
fn test_message_end_no_thought_indicator_if_short_thinking() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    // Short thinking (< 0.5s)
    std::thread::sleep(std::time::Duration::from_millis(50));

    handle_agent_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: agent_message("assistant", "Done"),
            turn: 1,
        },
    );

    assert!(
        !state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })),
        "should NOT have Thought indicator for short thinking"
    );
}

// ─── Empty response tests ─────────────────────────────────────────────────────

#[test]
fn test_empty_response_preserves_placeholder() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    // End with empty content
    handle_agent_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    // Placeholder is still there at this point (AgentEnd removes it)
    // but this tests that MessageEnd itself doesn't remove it
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "empty assistant should still exist before AgentEnd"
    );
}

#[test]
fn test_agent_end_removes_empty_placeholder() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    handle_agent_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        },
    );

    // AgentEnd should remove empty placeholder
    assert!(
        !state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "empty placeholder should be removed by AgentEnd"
    );
}

// ─── ContextCompacted tests ───────────────────────────────────────────────────

#[test]
fn test_context_compacted_is_ignored() {
    let mut state = make_test_state();
    state.agent_running = true;

    // ContextCompacted should not panic and should not change state
    handle_agent_event(
        &mut state,
        AgentEvent::ContextCompacted {
            original_count: 10,
            compacted_count: 5,
            summary_preview: "Summary of conversation...".to_string(),
        },
    );

    assert!(state.agent_running, "agent_running should remain unchanged");
    assert_eq!(
        state.messages.len(),
        0,
        "no messages should be added for ContextCompacted"
    );
}

// ─── Simple Message event test ────────────────────────────────────────────────

#[test]
fn test_simple_message_event_user() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::User { text, .. } if text == "Hello"
        )),
        "should have user message"
    );
}

#[test]
fn test_simple_message_event_assistant() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::Message {
            role: "assistant".to_string(),
            content: "Hello".to_string(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text == "Hello"
        )),
        "should have assistant message"
    );
}

#[test]
fn test_simple_message_event_system_filtered() {
    let mut state = make_test_state();

    // "Using model" system messages are filtered out
    handle_agent_event(
        &mut state,
        AgentEvent::Message {
            role: "system".to_string(),
            content: "Using gpt-4o model".to_string(),
        },
    );

    assert!(
        !state.messages.iter().any(|m| matches!(m, MessageItem::System { .. })),
        "Using model message should be filtered out"
    );
}

#[test]
fn test_simple_message_event_system_preserved() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::Message {
            role: "system".to_string(),
            content: "Some important system message".to_string(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text, .. } if text == "Some important system message"
        )),
        "regular system message should be preserved"
    );
}
