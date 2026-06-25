//! Tests for the /copy command.
//!
//! /copy now emits a `CopyToClipboard` event so the binary layer can
//! write the OSC 52 sequence directly to the terminal. These tests
//! verify the event is emitted with the correct payload.

use crate::Event;
use crate::model::{AppState, ChatMessage};
use crate::tests::fresh_state;
use std::sync::Mutex;

/// Serializes tests that touch shared env/state.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn copy_with_no_assistant_message_shows_error() {
    let mut state = fresh_state();
    // Open palette with '/'
    state.update(crate::Event::Input('/'));
    // Filter to 'copy' command
    state.update(crate::Event::PaletteFilter('c'));
    state.update(crate::Event::PaletteFilter('o'));
    state.update(crate::Event::PaletteFilter('p'));
    state.update(crate::Event::PaletteFilter('y'));
    // Select the copy command
    state.update(crate::Event::PaletteSelect);

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
            .content()
            .to_lowercase()
            .contains("no assistant"),
        "expected 'no assistant' message, got: {:?}",
        sys.last().unwrap().content()
    );
}

#[test]
fn copy_emits_clipboard_event_with_last_assistant_text() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage::assistant("the answer is 42")
        .with_id("resp.0")
        .with_timestamp(0.0));

    // Capture the event emitted by /copy. Since CopyToClipboard is
    // routed to control_event (which currently ignores it), we
    // observe it via the command handler directly.
    let result = state.handle_slash("/copy");
    assert!(
        matches!(result, Some(crate::commands::CommandResult::Event(crate::Event::CopyToClipboard(ref text))) if text == "the answer is 42"),
        "expected CopyToClipboard event with last assistant text, got {:?}",
        result
    );
}

#[test]
fn copy_uses_most_recent_assistant_message() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage::assistant("old response")
        .with_id("resp.0")
        .with_timestamp(0.0));
    state.session.messages.push(ChatMessage::assistant("newer response")
        .with_id("resp.1")
        .with_timestamp(1.0));

    let result = state.handle_slash("/copy");
    assert!(
        matches!(result, Some(crate::commands::CommandResult::Event(crate::Event::CopyToClipboard(ref text))) if text == "newer response"),
        "should copy the most recent assistant message, got {:?}",
        result
    );
}

#[test]
fn copy_event_payload_does_not_include_older_messages() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage::assistant("old response")
        .with_id("resp.0")
        .with_timestamp(0.0));
    state.session.messages.push(ChatMessage::assistant("newer response")
        .with_id("resp.1")
        .with_timestamp(1.0));

    let result = state.handle_slash("/copy");
    if let Some(crate::commands::CommandResult::Event(crate::Event::CopyToClipboard(text))) = result
    {
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
    state.session.messages.push(ChatMessage::user("hello world")
        .with_id("req.0")
        .with_timestamp(1.0));
    state.messages_changed();
    state.ensure_fresh();
    state.view.selected_post = Some(0);
    state
}

/// Build state with a single agent message post selected.
fn state_with_selected_post_agent() -> AppState {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage::assistant("the answer is 42")
        .with_id("resp.0")
        .with_timestamp(1.0));
    state.messages_changed();
    state.ensure_fresh();
    state.view.selected_post = Some(0);
    state
}

/// Build state with a tool-done element selected.
fn state_with_selected_post_tool_done() -> AppState {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage::assistant("running ls")
        .with_id("resp.0")
        .with_timestamp(1.0));
    state.session.messages.push(ChatMessage::tool_result("x bash 0.5s\nfile1\nfile2")
        .with_id("tool.0")
        .with_timestamp(2.0));
    state.messages_changed();
    state.ensure_fresh();
    // Tool-done is post index 1
    state.view.selected_post = Some(1);
    state
}

#[test]
fn copy_selected_post_text_user_message() {
    let mut state = state_with_selected_post_user();
    assert_eq!(
        state.copy_selected_post_text(),
        Some("hello world".into()),
        "y on a user message should copy its text"
    );
}

#[test]
fn copy_selected_post_text_agent_message() {
    let mut state = state_with_selected_post_agent();
    assert_eq!(
        state.copy_selected_post_text(),
        Some("the answer is 42".into()),
        "y on an agent message should copy its text"
    );
}

#[test]
fn copy_selected_post_text_tool_done() {
    let mut state = state_with_selected_post_tool_done();
    let text = state.copy_selected_post_text();
    assert!(text.is_some(), "y on a tool-done post should return Some");
    let text = text.unwrap();
    assert!(
        text.contains("ls") || text.contains("bash"),
        "tool-done text should include the command/tool name, got: {}",
        text
    );
}

#[test]
fn copy_selected_post_metadata_returns_timestamp() {
    let mut state = state_with_selected_post_agent();
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
    let mut state = AppState::default();
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
