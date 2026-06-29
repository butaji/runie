//! /copy slash command tests.

use super::exec;
use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::tests::fresh_state;

#[test]
fn copy_with_no_assistant_message_warns() {
    let mut state = fresh_state();
    exec(&mut state, "/copy");

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content().to_lowercase().contains("no assistant"),
        "expected 'no assistant' message: {}",
        last.content()
    );
}

#[test]
fn copy_emits_clipboard_event_with_last_assistant_text() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 0.0,
        id: "resp.0".into(),
        parts: vec![Part::Text {
            content: "the answer is 42".into(),
        }],
        ..Default::default()
    });

    let result = state.handle_slash("/copy");
    assert!(
        matches!(
            result,
            Some(crate::commands::CommandResult::Event(crate::Event::CopyToClipboard(ref text)))
            if text == "the answer is 42"
        ),
        "expected CopyToClipboard event with last assistant text, got {:?}",
        result
    );
}

#[test]
fn copy_uses_most_recent_assistant_message() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 0.0,
        id: "resp.0".into(),
        parts: vec![Part::Text {
            content: "old response".into(),
        }],
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 1.0,
        id: "resp.1".into(),
        parts: vec![Part::Text {
            content: "newer response".into(),
        }],
        ..Default::default()
    });

    let result = state.handle_slash("/copy");
    assert!(
        matches!(
            result,
            Some(crate::commands::CommandResult::Event(crate::Event::CopyToClipboard(ref text)))
            if text == "newer response"
        ),
        "should copy most recent assistant message, got {:?}",
        result
    );
}

#[test]
fn copy_round_trips_without_panic() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 0.0,
        id: "resp.0".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    exec(&mut state, "/copy");
    // Event is consumed silently by core; just verify no panic and dialog closed.
    assert!(state.open_dialog.is_none());
}
