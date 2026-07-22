#![allow(clippy::all)]
#![allow(clippy::too_many_lines)]
//! Layer 1 tests for agent streaming — lifecycle events populating `ChatMessage::parts`.
//!
//! Exercises `handle_llm_event` and `finish_turn` by feeding synthetic
//! `Event::TextStart` / `Event::ResponseDelta` / `Event::Done` sequences
//! into `AppState` without any TUI or async overhead.

#![allow(unused_imports)]
use crate::event::Event;
use crate::message::{ChatMessage, Part, Role};
use crate::model::AppState;
use crate::tool::{build_assistant_message, ParsedToolCall};

// -----------------------------------------------------------------------------
// Helpers (only used in tests, so dead_code is expected in lib check)
// -----------------------------------------------------------------------------

#[allow(dead_code)]
fn make_app_state() -> AppState {
    let mut state = AppState::default();
    let config = crate::config::Config::default();
    state.apply_config(&config);
    state
}

/// Feed a sequence of events into an AppState.
#[allow(dead_code)]
fn feed_events(state: &mut AppState, events: impl IntoIterator<Item = Event>) {
    for event in events {
        state.update(event);
    }
}

/// Return the last assistant message in the session.
#[allow(dead_code)]
fn last_assistant(state: &AppState) -> Option<&ChatMessage> {
    state
        .session
        .messages
        .iter()
        .rposition(|m| m.role == Role::Assistant)
        .and_then(|i| state.session.messages.get(i))
}

// ---------------------------------------------------------------------------
// Required task tests
// ---------------------------------------------------------------------------

/// TextStart → ResponseDelta("hi") → ResponseDelta(" there") → Done populates
/// a single Part::Text { content: "hi there" }.
#[test]
fn append_response_delta_populates_text_part() {
    let mut state = make_app_state();
    let id = "req-1".to_string();

    feed_events(
        &mut state,
        [
            Event::TextStart { id: id.clone() },
            Event::ResponseDelta { id: id.clone(), content: "hi".to_string() },
            Event::ResponseDelta { id: id.clone(), content: " there".to_string() },
            Event::TextEnd { id: id.clone() },
            Event::Done { id },
        ],
    );

    let msg = last_assistant(&state).expect("an assistant message should exist");
    let parts = &msg.parts;

    assert_eq!(
        parts.len(),
        1,
        "expected 1 part, got {}: {:?}",
        parts.len(),
        parts
    );
    assert!(
        matches!(&parts[0], Part::Text { content } if content == "hi there"),
        "expected Part::Text {{ content: \"hi there\" }}, got {:?}",
        parts[0]
    );
}

/// ThinkingStart → ThinkingDelta → ThinkingEnd populates a Part::Reasoning.
#[test]
fn append_response_delta_populates_reasoning_part() {
    let mut state = make_app_state();
    let id = "req-2".to_string();

    feed_events(
        &mut state,
        [
            Event::ThinkingStart { id: id.clone() },
            Event::ThinkingDelta { id: id.clone(), content: "reasoning".to_string() },
            Event::ThinkingEnd { id },
        ],
    );

    let msg = last_assistant(&state).expect("assistant message should exist after ThinkingStart");
    let parts = &msg.parts;

    assert_eq!(
        parts.len(),
        1,
        "expected 1 part, got {}: {:?}",
        parts.len(),
        parts
    );
    assert!(
        matches!(&parts[0], Part::Reasoning { content } if content == "reasoning"),
        "expected Part::Reasoning {{ content: \"reasoning\" }}, got {:?}",
        parts[0]
    );
}

/// Two separate text cycles produce two Part::Text entries across two messages.
#[test]
fn append_response_delta_multiple_text_blocks() {
    let mut state = make_app_state();
    let id1 = "req-3a".to_string();
    let id2 = "req-3b".to_string();

    // First text block.
    feed_events(
        &mut state,
        [
            Event::TextStart { id: id1.clone() },
            Event::ResponseDelta { id: id1.clone(), content: "a".to_string() },
            Event::TextEnd { id: id1.clone() },
            Event::Done { id: id1 },
        ],
    );

    // Second text block (new request).
    feed_events(
        &mut state,
        [
            Event::TextStart { id: id2.clone() },
            Event::ResponseDelta { id: id2.clone(), content: "b".to_string() },
            Event::TextEnd { id: id2.clone() },
            Event::Done { id: id2 },
        ],
    );

    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();

    assert_eq!(assistants.len(), 2, "expected 2 assistant messages");

    assert!(
        matches!(&assistants[0].parts[..], [Part::Text { content }] if content == "a"),
        "first message parts should be [Text {{ \"a\" }}], got {:?}",
        assistants[0].parts
    );

    assert!(
        matches!(&assistants[1].parts[..], [Part::Text { content }] if content == "b"),
        "second message parts should be [Text {{ \"b\" }}], got {:?}",
        assistants[1].parts
    );
}

/// TextStart + ResponseDelta without TextEnd: finish_turn closes the open
/// Part::Text from accumulated streaming tail.
#[test]
fn finish_turn_closes_open_parts() {
    let mut state = make_app_state();
    let id = "req-4".to_string();

    // TextStart begins a Part::Text; ResponseDelta accumulates "hi".
    // No TextEnd received before Done.
    feed_events(
        &mut state,
        [Event::TextStart { id: id.clone() }, Event::ResponseDelta { id: id.clone(), content: "hi".to_string() }],
    );

    let msg_before = last_assistant(&state).expect("assistant message should exist after ResponseDelta");
    assert!(
        !msg_before.parts.is_empty(),
        "parts should not be empty (TextStart pushes a Part::Text)"
    );

    // Done → finish_turn → close_open_parts appends remaining tail.
    feed_events(&mut state, [Event::Done { id }]);

    let msg_after = last_assistant(&state).expect("assistant message should still exist");
    assert!(
        matches!(&msg_after.parts[..], [Part::Text { content }] if content == "hi"),
        "expected one Part::Text {{ content: \"hi\" }}, got {:?}",
        msg_after.parts
    );
}

/// ResponseDelta without any lifecycle events: finish_turn fallback creates
/// Part::Text from accumulated message content.
#[test]
fn finish_turn_fallback_creates_text_part_when_empty() {
    let mut state = make_app_state();
    let id = "req-5".to_string();

    feed_events(
        &mut state,
        [Event::ResponseDelta { id: id.clone(), content: "hi".to_string() }],
    );
    feed_events(&mut state, [Event::Done { id }]);

    let msg = last_assistant(&state).expect("assistant message should exist");
    assert!(
        matches!(&msg.parts[..], [Part::Text { content }] if content == "hi"),
        "expected one Part::Text {{ content: \"hi\" }}, got {:?}",
        msg.parts
    );
}

/// ResponseDelta without TextStart goes through the streaming buffer path.
/// Newlines inside the streamed content must survive into the stored
/// message (live MiniMax bug: "Reasoning\nMethod" rendered as
/// "ReasoningMethod").
#[test]
fn response_delta_buffered_path_preserves_newlines() {
    let mut state = make_app_state();
    let id = "req-nl-1".to_string();

    feed_events(
        &mut state,
        [
            Event::ResponseDelta { id: id.clone(), content: "first line\nsecond line\nthird line\n".to_string() },
            Event::ResponseDelta { id: id.clone(), content: "      indented tail".to_string() },
            Event::Done { id },
        ],
    );

    let msg = last_assistant(&state).expect("assistant message should exist");
    assert_eq!(
        msg.content(),
        "first line\nsecond line\nthird line\n      indented tail",
        "stored message lost newlines: {:?}",
        msg.content()
    );
}

/// build_assistant_message appends a ToolCall to existing parts without
/// overwriting the text and reasoning parts already streamed in.
#[test]
#[allow(clippy::cognitive_complexity)]
fn build_assistant_message_appends_tool_call_to_existing_parts() {
    let existing_text = "Let me check";
    let reasoning_text = "searching for the right approach";

    let tools = vec![ParsedToolCall {
        name: "list_dir".to_string(),
        args: serde_json::json!({ "path": "." }),
        id: Some("call_1".to_string()),
    }];

    let msg = build_assistant_message(existing_text, Some(&reasoning_text), &tools);

    assert_eq!(msg.content(), existing_text);
    let tool_calls = msg.tool_calls();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_1");
    assert_eq!(tool_calls[0].name, "list_dir");

    // Parts: Text + Reasoning + ToolCall.
    assert_eq!(msg.parts.len(), 3);
    assert!(
        matches!(&msg.parts[0], Part::Text { content } if content == existing_text),
        "expected Part::Text {{ content: {:?} }}, got {:?}",
        existing_text,
        msg.parts[0]
    );
    assert!(
        matches!(&msg.parts[1], Part::Reasoning { content } if content == reasoning_text),
        "expected Part::Reasoning {{ content: {:?} }}, got {:?}",
        reasoning_text,
        msg.parts[1]
    );
    assert!(
        matches!(
            &msg.parts[2],
            Part::ToolCall { id, name, .. }
            if id == "call_1" && name == "list_dir"
        ),
        "expected Part::ToolCall {{ id: \"call_1\", name: \"list_dir\" }}, got {:?}",
        msg.parts[2]
    );
}
