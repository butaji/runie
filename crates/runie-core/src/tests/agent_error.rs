//! Tests for agent error handling — especially that errors clear the active
//! turn so the UI does not show a stuck "Working..." status.

use crate::Event;
use crate::model::{AppState, QueuedMessage, QueuedMessageKind};

#[test]
fn agent_error_clears_turn_active() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.streaming = true;
    state.agent.inflight = 1;

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error: Missing API key".to_string(),
    });

    assert!(!state.agent.turn_active, "turn_active should be cleared");
    assert!(!state.agent.streaming, "streaming should be cleared");
    assert_eq!(state.agent.inflight, 0, "inflight should be reset");
}

#[test]
fn agent_error_resets_timers() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.agent.thinking_started_at = Some(std::time::Instant::now());
    state.agent.tool_started_at = Some(std::time::Instant::now());

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error".to_string(),
    });

    assert!(state.agent.turn_started_at.is_none());
    assert!(state.agent.thinking_started_at.is_none());
    assert!(state.agent.tool_started_at.is_none());
}

#[test]
fn agent_error_inserts_error_message() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Missing API key".to_string(),
    });

    let error_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.content().contains("Error: Missing API key"));
    assert!(error_msg.is_some(), "error message should be recorded");
}

#[test]
fn agent_error_clears_current_request_id() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.current_request_id = Some("req.0".to_string());

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error".to_string(),
    });

    assert!(state.agent.current_request_id.is_none());
}

#[test]
fn agent_error_clears_streaming_and_thought_state() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_tokens_out = 42;
    state.agent.intermediate_step_count = 3;
    state.agent.thought_seq = 7;
    state.agent.last_assistant_index = Some(2);
    state.view.vim_nav_pending = true;

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error".to_string(),
    });

    assert_eq!(state.agent.turn_tokens_out, 0);
    assert_eq!(state.agent.intermediate_step_count, 0);
    assert_eq!(state.agent.thought_seq, 0);
    assert!(state.agent.last_assistant_index.is_none());
    assert!(!state.view.vim_nav_pending);
}

#[test]
fn agent_error_resets_streaming_buffer() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    // An unclosed code fence leaves pending content in the buffer tail.
    state.agent.streaming_buffer.push_delta("```rust\npartial");
    assert!(state.agent.streaming_buffer.has_pending_content());

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error".to_string(),
    });

    assert!(!state.agent.streaming_buffer.has_pending_content());
}

#[test]
fn agent_error_delivers_queued_messages() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.message_queue.push(QueuedMessage {
        content: "follow up".to_string(),
        kind: QueuedMessageKind::FollowUp,
    });

    state.update(crate::Event::Error {
        id: "req.0".to_string(),
        message: "Provider error".to_string(),
    });

    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.agent.request_queue.len(), 1);
    assert_eq!(state.agent.request_queue.front().map(|(c, _)| c), Some(&"follow up".to_string()));
}
