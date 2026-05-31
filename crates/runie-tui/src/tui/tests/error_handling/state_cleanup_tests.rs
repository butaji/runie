//! State cleanup tests.
//!
//! Tests for on_agent_error() state management:
//! - Error clears agent_running
//! - Error clears agent_start_time
//! - Error clears input_right_info
//! - Error sets status_header="Error"
//! - Error in Permission mode resets to Chat
//! - Error in Onboarding keeps Onboarding
//! - Error removes empty placeholder
//! - Error preserves non-empty assistant
//! - Error displayed in feed

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::agent::error::on_agent_error;
use crate::tui::tests::reducer::make_state;
use std::time::Instant;

fn make_state_with_agent() -> AppState {
    let mut state = make_state();
    state.agent_running = true;
    state.agent_start_time = Some(Instant::now());
    state.input_right_info = "token_count".to_string();
    state
}

#[test]
fn test_error_clears_agent_running() {
    let mut state = make_state_with_agent();
    assert!(state.agent_running, "agent should be running before error");

    on_agent_error(&mut state, "Test error".to_string());

    assert!(
        !state.agent_running,
        "agent_running should be cleared on error"
    );
}

#[test]
fn test_error_clears_agent_start_time() {
    let mut state = make_state_with_agent();
    assert!(
        state.agent_start_time.is_some(),
        "agent_start_time should be set before error"
    );

    on_agent_error(&mut state, "Test error".to_string());

    assert!(
        state.agent_start_time.is_none(),
        "agent_start_time should be cleared on error"
    );
}

#[test]
fn test_error_clears_input_right_info() {
    let mut state = make_state_with_agent();
    assert!(
        !state.input_right_info.is_empty(),
        "input_right_info should be set before error"
    );

    on_agent_error(&mut state, "Test error".to_string());

    assert!(
        state.input_right_info.is_empty(),
        "input_right_info should be cleared on error"
    );
}

#[test]
fn test_error_sets_status_header() {
    let mut state = make_state_with_agent();
    state.status_header = Some("Thinking".to_string());

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.status_header,
        Some("Error".to_string()),
        "status_header should be set to Error"
    );
}

#[test]
fn test_error_in_permission_mode_resets_to_chat() {
    let mut state = make_state_with_agent();
    state.mode = TuiMode::Permission;

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Chat,
        "Mode should reset to Chat on error (from Permission)"
    );
}

#[test]
fn test_error_in_onboarding_keeps_onboarding() {
    let mut state = make_state_with_agent();
    state.mode = TuiMode::Onboarding;

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Onboarding,
        "Mode should stay in Onboarding on error"
    );
}

#[test]
fn test_error_removes_empty_placeholder() {
    let mut state = make_state();
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: None,
        timestamp: None,
    });

    on_agent_error(&mut state, "Test error".to_string());

    // Empty assistant placeholder should be removed
    let has_empty_assistant = state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()));
    assert!(
        !has_empty_assistant,
        "Empty assistant placeholder should be removed on error"
    );
}

#[test]
fn test_error_preserves_non_empty_assistant() {
    let mut state = make_state();
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi there!".to_string(),
        model: None,
        timestamp: None,
    });

    on_agent_error(&mut state, "Test error".to_string());

    // Non-empty assistant should be preserved
    let assistant_count = state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(
        assistant_count, 1,
        "Non-empty assistant should be preserved on error"
    );
}

#[test]
fn test_error_displayed_in_feed() {
    let mut state = make_state();

    on_agent_error(&mut state, "Test error message".to_string());

    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "Error should be displayed in message feed"
    );
}

#[test]
fn test_error_message_includes_sanitized_content() {
    let mut state = make_state();

    on_agent_error(&mut state, "Connection timeout".to_string());

    if let Some(MessageItem::Error { message, .. }) = state.messages.last() {
        assert!(
            message.contains("Connection timeout"),
            "Error message should contain the sanitized error text"
        );
    } else {
        panic!("Expected error message in state");
    }
}

#[test]
fn test_error_sets_status_details() {
    let mut state = make_state();

    on_agent_error(&mut state, "Connection timeout".to_string());

    assert!(
        state.status_details.is_some(),
        "status_details should be set on error"
    );
}

#[test]
fn test_error_sets_status_start_time() {
    let mut state = make_state();

    on_agent_error(&mut state, "Test error".to_string());

    assert!(
        state.status_start_time.is_some(),
        "status_start_time should be set on error"
    );
}

#[test]
fn test_error_does_not_remove_user_message() {
    let mut state = make_state();
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });

    on_agent_error(&mut state, "Test error".to_string());

    let user_count = state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .count();
    assert_eq!(
        user_count, 1,
        "User message should be preserved on error"
    );
}

#[test]
fn test_error_with_long_message_gets_truncated() {
    let mut state = make_state();
    let long_error = "Error: ".to_string() + &"x".repeat(1000);

    on_agent_error(&mut state, long_error.clone());

    if let Some(MessageItem::Error { message, .. }) = state.messages.last() {
        assert!(
            message.len() < long_error.len(),
            "Long error should be truncated in feed"
        );
    } else {
        panic!("Expected error message in state");
    }
}

#[test]
fn test_error_in_diff_viewer_mode_resets_to_chat() {
    let mut state = make_state_with_agent();
    state.mode = TuiMode::DiffViewer;

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Chat,
        "Mode should reset to Chat on error (from DiffViewer)"
    );
}

#[test]
fn test_error_in_session_tree_mode_resets_to_chat() {
    let mut state = make_state_with_agent();
    state.mode = TuiMode::SessionTree;

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Chat,
        "Mode should reset to Chat on error (from SessionTree)"
    );
}

#[test]
fn test_error_in_overlay_mode_resets_to_chat() {
    let mut state = make_state_with_agent();
    state.mode = TuiMode::Overlay;

    on_agent_error(&mut state, "Test error".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Chat,
        "Mode should reset to Chat on error (from Overlay)"
    );
}
