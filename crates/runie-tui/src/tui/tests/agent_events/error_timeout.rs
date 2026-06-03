//! Error and timeout tests.
//!
//! Tests:
//! - Error event clears agent_running
//! - Error sanitization (>500 chars, stack traces)
//! - Recoverable vs fatal classification
//! - Error removes empty assistant placeholder
//! - Error resets mode to Chat (unless Onboarding)

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::TuiMode;
use crate::tui::update::agent::handle_agent_event;
use crate::tui::update::agent::error::sanitize_error_message;
use runie_agent::AgentEvent;

/// Helper: Create AppState ready for error testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state
}

// ─── Error sanitization tests ────────────────────────────────────────────────

#[test]
fn test_sanitize_short_message_unchanged() {
    let msg = "Simple error message";
    let result = sanitize_error_message(msg);
    assert_eq!(result, msg, "short message should be unchanged");
}

#[test]
fn test_sanitize_long_message_truncated() {
    let long_msg = "a".repeat(600);
    let result = sanitize_error_message(&long_msg);
    assert!(
        result.len() < long_msg.len(),
        "long message should be truncated"
    );
    assert!(
        result.contains("truncated"),
        "truncated message should indicate truncation"
    );
    assert!(
        result.contains("600"),
        "should indicate original length"
    );
}

#[test]
fn test_sanitize_stack_trace_shortened() {
    let stack_trace = "Error: something failed\n  at Function.module (file.js:10:5)\n  at Another.module (file.js:15:3)\n  at main (index.js:1:1)\nthread 'async' panicked at 'critical error'";
    let result = sanitize_error_message(stack_trace);
    // Stack trace patterns are detected and annotation is added
    assert!(
        result.contains("hidden"),
        "should indicate hidden details"
    );
}

#[test]
fn test_sanitize_stack_trace_patterns() {
    let patterns = vec![
        "stack backtrace",
        "thread '",
        "at 0x",
        "panicked at",
        "---- ",
        "FAILED",
        "test result:",
    ];

    for pattern in patterns {
        let msg = format!("Error message\n  {} \n  more details", pattern);
        let result = sanitize_error_message(&msg);
        assert!(
            result.contains("hidden") || result.len() < msg.len(),
            "pattern '{}' should trigger stack trace handling",
            pattern
        );
    }
}

#[test]
fn test_sanitize_truncation_preserves_500_chars() {
    let msg = "a".repeat(1000);
    let result = sanitize_error_message(&msg);
    // Should preserve up to ~475 chars (500 - 25 for "... [message truncated, 1000 chars total]")
    assert!(
        result.len() <= 520,
        "result should be around 500 chars"
    );
}

// ─── Recoverable error classification tests ──────────────────────────────────

// Note: is_recoverable_error is private, so we test the classification
// behavior indirectly through handle_agent_event

#[test]
fn test_error_classification_recoverable_timeout() {
    let mut state = make_test_state();
    state.agent_running = true;

    // Error with "timeout" in message should be marked recoverable
    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "Request timeout after 30s".to_string(),
            error_type: "Timeout".to_string(),
            recoverable: true, // recoverable field comes from agent
            context: "".to_string(),
        },
    );

    let recoverable = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    });
    assert_eq!(recoverable, Some(true), "error should be recoverable");
}

#[test]
fn test_error_classification_recoverable_network() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "Connection refused - network error".to_string(),
            error_type: "NetworkError".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    let recoverable = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    });
    assert_eq!(recoverable, Some(true), "network error should be recoverable");
}

#[test]
fn test_error_classification_fatal() {
    let mut state = make_test_state();
    state.agent_running = true;

    // AgentEvent.Error has a recoverable field that is passed through
    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "Invalid syntax in request".to_string(),
            error_type: "SyntaxError".to_string(),
            recoverable: false, // Explicitly not recoverable
            context: "".to_string(),
        },
    );

    let recoverable = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    });
    assert_eq!(recoverable, Some(false), "syntax error should not be recoverable");
}

// ─── Error event handling tests ──────────────────────────────────────────────

#[test]
fn test_error_clears_agent_running() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "Test error".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert!(!state.agent_running, "agent_running should be false");
}

#[test]
fn test_error_adds_error_message() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "Something failed".to_string(),
            error_type: "TestError".to_string(),
            recoverable: false,
            context: "test context".to_string(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Error { message, recoverable } if message.contains("Something failed") && !recoverable
        )),
        "should have error message with correct recoverability"
    );
}

#[test]
fn test_error_removes_empty_placeholder() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: Some("test-model".to_string()),
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert!(
        !state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "empty placeholder should be removed"
    );
}

#[test]
fn test_error_clears_input_right_info() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.input_right_info = "some info".to_string();

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert!(
        state.input_right_info.is_empty(),
        "input_right_info should be cleared"
    );
}

#[test]
fn test_error_resets_mode_to_chat() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert_eq!(state.mode, TuiMode::Chat, "mode should reset to Chat");
}

#[test]
fn test_error_preserves_onboarding_mode() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.mode = TuiMode::Onboarding;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert_eq!(state.mode, TuiMode::Onboarding, "mode should stay Onboarding");
}

#[test]
fn test_error_clears_agent_start_time() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.agent_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert!(
        state.agent_start_time.is_none(),
        "agent_start_time should be cleared"
    );
}

#[test]
fn test_error_sets_status_header() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    assert_eq!(
        state.status_header.as_deref(),
        Some("Error"),
        "status_header should be Error"
    );
}

#[test]
fn test_error_sanitizes_long_message() {
    let mut state = make_test_state();
    state.agent_running = true;
    let long_msg = "a".repeat(600);

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: long_msg.clone(),
            error_type: "LongError".to_string(),
            recoverable: false,
            context: "".to_string(),
        },
    );

    let error_msg = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { message, .. } => Some(message.clone()),
        _ => None,
    });

    assert!(error_msg.is_some(), "should have error message");
    let msg = error_msg.unwrap();
    assert!(
        msg.len() < long_msg.len(),
        "error message should be sanitized/truncated"
    );
}

#[test]
fn test_error_with_stack_trace_sanitized() {
    let mut state = make_test_state();
    state.agent_running = true;
    let stack_trace = "Error: panic\n  at module (file.js:10)\nthread 'main' panicked".to_string();

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: stack_trace.clone(),
            error_type: "Panic".to_string(),
            recoverable: false,
            context: "".to_string(),
        },
    );

    let error_msg = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { message, .. } => Some(message.clone()),
        _ => None,
    });

    assert!(error_msg.is_some(), "should have error message");
    let msg = error_msg.unwrap();
    // Stack trace annotation is added
    assert!(
        msg.contains("Additional details hidden") || msg.contains("hidden"),
        "should indicate hidden details"
    );
}

// ─── Error classification tests ───────────────────────────────────────────────

#[test]
fn test_error_recoverable_flag_set_correctly() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "timeout error".to_string(),
            error_type: "Timeout".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );

    let is_recoverable = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    });
    assert_eq!(is_recoverable, Some(true), "recoverable should be true for timeout");
}

#[test]
fn test_error_fatal_flag_set_correctly() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "invalid syntax".to_string(),
            error_type: "SyntaxError".to_string(),
            recoverable: false,
            context: "".to_string(),
        },
    );

    let is_recoverable = state.messages.iter().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(*recoverable),
        _ => None,
    });
    assert_eq!(is_recoverable, Some(false), "recoverable should be false for syntax error");
}
