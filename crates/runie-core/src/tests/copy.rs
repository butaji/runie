//! Tests for the /copy command.
//!
//! /copy now emits a `CopyToClipboard` event so the binary layer can
//! write the OSC 52 sequence directly to the terminal. These tests
//! verify the event is emitted with the correct payload.

use crate::event::Event;
use crate::model::AppState;
use std::sync::Mutex;

/// Serializes tests that touch shared env/state.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fresh_state() -> AppState {
    AppState::default()
}

fn type_str(state: &mut AppState, s: &str) {
    for c in s.chars() {
        state.update(Event::Input(c));
    }
}

#[test]
fn copy_with_no_assistant_message_shows_error() {
    let mut state = fresh_state();
    type_str(&mut state, "/copy");
    state.update(Event::Submit);

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system message");
    assert!(
        sys.last()
            .unwrap()
            .content
            .to_lowercase()
            .contains("no assistant"),
        "expected 'no assistant' message, got: {:?}",
        sys.last().unwrap().content
    );
}

#[test]
fn copy_emits_clipboard_event_with_last_assistant_text() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "the answer is 42".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });

    // Capture the event emitted by /copy. Since CopyToClipboard is
    // routed to control_event (which currently ignores it), we
    // observe it via the command handler directly.
    let result = state.handle_slash("/copy");
    assert!(
        matches!(result, Some(crate::commands::CommandResult::Event(Event::CopyToClipboard(ref text))) if text == "the answer is 42"),
        "expected CopyToClipboard event with last assistant text, got {:?}",
        result
    );
}

#[test]
fn copy_uses_most_recent_assistant_message() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "old response".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "newer response".into(),
        timestamp: 1.0,
        id: "resp.1".into(),
        ..Default::default()
    });

    let result = state.handle_slash("/copy");
    assert!(
        matches!(result, Some(crate::commands::CommandResult::Event(Event::CopyToClipboard(ref text))) if text == "newer response"),
        "should copy the most recent assistant message, got {:?}",
        result
    );
}

#[test]
fn copy_event_payload_does_not_include_older_messages() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "old response".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "newer response".into(),
        timestamp: 1.0,
        id: "resp.1".into(),
        ..Default::default()
    });

    let result = state.handle_slash("/copy");
    if let Some(crate::commands::CommandResult::Event(Event::CopyToClipboard(text))) = result {
        assert!(
            !text.contains("old response"),
            "should NOT copy older messages, got: {:?}",
            text
        );
    } else {
        panic!("expected CopyToClipboard event, got {:?}", result);
    }
}
