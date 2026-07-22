//! Tests for agent error handling — especially that errors clear the active
//! turn so the UI does not show a stuck "Working..." status.

use crate::model::{AppState, QueuedMessage, QueuedMessageKind};
use crate::view::{Element, LazyCache};

fn feed_elements(state: &AppState) -> Vec<Element> {
    LazyCache::feed(state).elements
}

fn feed_has_error(state: &AppState, needle: &str) -> bool {
    feed_elements(state)
        .iter()
        .any(|e| matches!(e, Element::AgentMessage { content, .. } if content.contains(needle)))
}

fn feed_has_turn_complete(state: &AppState) -> bool {
    feed_elements(state)
        .iter()
        .any(|e| matches!(e, Element::TurnComplete { .. }))
}

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

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Provider error".to_string() });

    assert!(state.agent.turn_started_at.is_none());
    assert!(state.agent.thinking_started_at.is_none());
    assert!(state.agent.tool_started_at.is_none());
}

#[test]
fn agent_error_inserts_error_message() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Missing API key".to_string() });

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

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Provider error".to_string() });

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

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Provider error".to_string() });

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

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Provider error".to_string() });

    assert!(!state.agent.streaming_buffer.has_pending_content());
}

#[test]
fn agent_error_delivers_queued_messages() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    // Push to agent state message queue.
    state
        .agent_state_mut()
        .message_queue
        .push(QueuedMessage { content: "follow up".to_string(), kind: QueuedMessageKind::FollowUp });

    state.update(crate::Event::Error { id: "req.0".to_string(), message: "Provider error".to_string() });

    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.agent.request_queue.len(), 1);
    assert_eq!(
        state.agent.request_queue.front().map(|(c, _)| c),
        Some(&"follow up".to_string())
    );
}

/// Regression for the live MiniMax bug: a turn streamed reasoning (rendered
/// as "◆ Thought for 2.2s"), the provider stream then failed, and the feed
/// showed NOTHING afterwards — no error element, no completion marker —
/// even though the turn was over (input box back, status idle).
///
/// The error must survive the full [reasoning → thought → error → done]
/// event sequence as a visible feed element.
#[test]
fn agent_error_after_streamed_reasoning_stays_in_feed() {
    let mut state = AppState::default();
    state.apply_config(&crate::config::Config::default());
    let id = "req.0".to_string();

    // MiniMax-style turn: reasoning streams inside content deltas (the
    // think filter holds it back, so no assistant message exists yet),
    // then the stream fails — e.g. a 429 surfaced after reasoning.
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: "<think>analyzing the request".into() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });
    state.update(crate::Event::Error { id: id.clone(), message: "Agent error: Rate limited".into() });
    state.update(crate::Event::Done { id: id.clone() });

    assert!(
        feed_has_error(&state, "Error: Agent error: Rate limited"),
        "error element must be visible in the feed; elements: {:?}",
        feed_elements(&state)
    );
    // The turn is over: terminal state must reflect that.
    assert!(!state.agent_state().streaming, "streaming must be cleared");
    assert!(
        !state.agent_state().turn_active,
        "turn_active must be cleared"
    );
}

/// Any turn that ends — success OR error — must leave a visible terminal
/// marker in the feed: an error element and/or a "Turn completed" marker.
#[test]
fn errored_turn_leaves_terminal_marker_in_feed() {
    let mut state = AppState::default();
    state.apply_config(&crate::config::Config::default());
    let id = "req.0".to_string();

    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::Error { id: id.clone(), message: "Agent error: Rate limited".into() });
    state.update(crate::Event::Done { id: id.clone() });

    let has_error = feed_has_error(&state, "Error: Agent error: Rate limited");
    let has_turn_complete = feed_has_turn_complete(&state);
    assert!(
        has_error || has_turn_complete,
        "errored turn must leave a terminal marker (error element or Turn \
         completed); elements: {:?}",
        feed_elements(&state)
    );
    assert!(
        has_error,
        "the error itself must render as a feed element; elements: {:?}",
        feed_elements(&state)
    );
}
