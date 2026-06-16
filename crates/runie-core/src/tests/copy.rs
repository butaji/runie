//! Tests for the /copy command.
//!
//! /copy now emits a `CopyToClipboard` event so the binary layer can
//! write the OSC 52 sequence directly to the terminal. These tests
//! verify the event is emitted with the correct payload.

use crate::event::{DialogEvent, InputEvent};
use crate::model::{AppState, ChatMessage};
use std::sync::Mutex;

/// Serializes tests that touch shared env/state.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fresh_state() -> AppState {
    AppState::default()
}

fn type_str(state: &mut AppState, s: &str) {
    for c in s.chars() {
        state.update(InputEvent::Input(c));
    }
}

#[test]
fn copy_with_no_assistant_message_shows_error() {
    let mut state = fresh_state();
    // Open palette with '/'
    state.update(InputEvent::Input('/'));
    // Filter to 'copy' command
    state.update(DialogEvent::PaletteFilter('c'));
    state.update(DialogEvent::PaletteFilter('o'));
    state.update(DialogEvent::PaletteFilter('p'));
    state.update(DialogEvent::PaletteFilter('y'));
    // Select the copy command
    state.update(DialogEvent::PaletteSelect);

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
        matches!(result, Some(crate::commands::CommandResult::Event(DialogEvent::CopyToClipboard(ref text))) if text == "the answer is 42"),
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
        matches!(result, Some(crate::commands::CommandResult::Event(DialogEvent::CopyToClipboard(ref text))) if text == "newer response"),
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
    if let Some(crate::commands::CommandResult::Event(DialogEvent::CopyToClipboard(text))) = result {
        assert!(
            !text.contains("old response"),
            "should NOT copy older messages, got: {:?}",
            text
        );
    } else {
        panic!("expected CopyToClipboard event, got {:?}", result);
    }
}

// ── Block copy (vim y / Y) ─────────────────────────────────────────────────────

/// Build state with a single user message post selected.
fn state_with_selected_post_user() -> AppState {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: crate::model::Role::User,
        content: "hello world".into(),
        timestamp: 1.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.selected_post = Some(0);
    state
}

/// Build state with a single agent message post selected.
fn state_with_selected_post_agent() -> AppState {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: crate::model::Role::Assistant,
        content: "the answer is 42".into(),
        timestamp: 1.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.selected_post = Some(0);
    state
}

/// Build state with a tool-done element selected.
fn state_with_selected_post_tool_done() -> AppState {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: crate::model::Role::Assistant,
        content: "running ls".into(),
        timestamp: 1.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: crate::model::Role::Tool,
        content: "x bash 0.5s\nfile1\nfile2".into(),
        timestamp: 2.0,
        id: "tool.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    // Tool-done is post index 1
    state.view.selected_post = Some(1);
    state
}

#[test]
fn copy_selected_post_text_user_message() {
    let state = state_with_selected_post_user();
    assert_eq!(
        state.copy_selected_post_text(),
        Some("hello world".into()),
        "y on a user message should copy its text"
    );
}

#[test]
fn copy_selected_post_text_agent_message() {
    let state = state_with_selected_post_agent();
    assert_eq!(
        state.copy_selected_post_text(),
        Some("the answer is 42".into()),
        "y on an agent message should copy its text"
    );
}

#[test]
fn copy_selected_post_text_tool_done() {
    let state = state_with_selected_post_tool_done();
    let text = state.copy_selected_post_text();
    assert!(
        text.is_some(),
        "y on a tool-done post should return Some"
    );
    let text = text.unwrap();
    assert!(
        text.contains("ls") || text.contains("bash"),
        "tool-done text should include the command/tool name, got: {}",
        text
    );
}

#[test]
fn copy_selected_post_metadata_returns_timestamp() {
    let state = state_with_selected_post_agent();
    let meta = state.copy_selected_post_metadata();
    assert!(
        meta.is_some(),
        "Y on an agent message should return metadata"
    );
    let meta = meta.unwrap();
    assert!(
        meta.contains('1'),
        "metadata should include the timestamp, got: {}",
        meta
    );
}

#[test]
fn copy_selected_post_text_no_selection_returns_none() {
    let state = AppState::default();
    assert_eq!(state.copy_selected_post_text(), None);
}

#[test]
fn copy_selected_post_text_empty_post_returns_none() {
    // A post with only Spacer elements returns None (Spacer has no text)
    let mut state = AppState::default();
    // Empty state has one empty post
    state.messages_changed();
    state.ensure_fresh();
    state.view.selected_post = Some(0);
    assert_eq!(state.copy_selected_post_text(), None);
}
