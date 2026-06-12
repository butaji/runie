//! Tests for the /copy command.
//!
//! Regression: previously `/copy` claimed success but did nothing. Now it
//! persists the last assistant message to a file the user can paste from.

use crate::event::Event;
use crate::model::AppState;
use std::sync::Mutex;

/// Serializes tests that touch the `RUNIE_CACHE_DIR` env var.
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
fn copy_writes_last_assistant_text_to_clipboard_file() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("runie_copy_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::env::set_var("RUNIE_CACHE_DIR", &tmp);
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "the answer is 42".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    type_str(&mut state, "/copy");
    state.update(Event::Submit);
    let clip = tmp.join("clipboard.md");
    assert!(clip.exists(), "clipboard file should exist at {:?}", clip);
    let content = std::fs::read_to_string(&clip).unwrap();
    assert!(
        content.contains("the answer is 42"),
        "clipboard file should contain assistant text, got: {:?}",
        content
    );
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    let last = sys.last().expect("system message after /copy");
    assert!(
        last.content.contains("clipboard.md"),
        "system message should mention the file path, got: {:?}",
        last.content
    );
    let _ = std::fs::remove_dir_all(&tmp);
    std::env::remove_var("RUNIE_CACHE_DIR");
}

#[test]
fn copy_uses_most_recent_assistant_message() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("runie_copy_recent_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::env::set_var("RUNIE_CACHE_DIR", &tmp);
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
    type_str(&mut state, "/copy");
    state.update(Event::Submit);
    let clip = tmp.join("clipboard.md");
    let content = std::fs::read_to_string(&clip).unwrap();
    assert!(
        content.contains("newer response"),
        "should copy the most recent, got: {:?}",
        content
    );
    assert!(
        !content.contains("old response"),
        "should NOT copy older messages"
    );
    let _ = std::fs::remove_dir_all(&tmp);
    std::env::remove_var("RUNIE_CACHE_DIR");
}

#[test]
fn copy_uses_default_cache_dir_when_env_unset() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("RUNIE_CACHE_DIR");

    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Assistant,
        content: "default dir test".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });

    type_str(&mut state, "/copy");
    state.update(Event::Submit);

    // Don't assert the actual path (depends on $HOME); just assert the
    // system message contains a path-like string.
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content.contains("clipboard.md"),
        "system message should mention the file path, got: {:?}",
        last.content
    );
}
