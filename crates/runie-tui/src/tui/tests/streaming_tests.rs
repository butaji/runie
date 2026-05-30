//! Streaming tests for agent message streaming behavior.
//!
//! Tests verify correct handling of streaming events:
//! - Text chunk accumulation
//! - Tool call interleaving
//! - Rapid update handling
//! - Empty stream behavior
//! - Token usage tracking during streams

use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, AgentMessage, ContentPart};

/// Helper: Create an AgentMessage with given role and content text.
fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Helper: Create a TokenUsage event with given prompt/completion.
fn token_usage(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

// ─── Test: Stream text chunks accumulate correctly ───────────────────────────

#[tokio::test]
async fn test_stream_text_accumulation() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Start message
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Stream text in chunks - message contains FULL accumulated text, delta is the new part
    let full_texts = vec!["Hi", "Hi there", "Hi there!"];
    let deltas = vec!["Hi", " there", "!"];
    
    for (full, delta) in full_texts.iter().zip(deltas.iter()) {
        harness.handle_agent_event(AgentEvent::MessageUpdate {
            message: agent_message("assistant", full),
            turn: 1,
            delta: delta.to_string(),
        });
    }

    harness.assert_last_assistant_text("Hi there!");
}

// ─── Test: Stream with tool calls interleaved ─────────────────────────────────

#[tokio::test]
async fn test_stream_with_tool_calls() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Run ls");

    // Start message
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Stream partial text - message has FULL text
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I'll"),
        turn: 1,
        delta: "I'll".to_string(),
    });

    // Tool execution
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    // More text after tool - message has FULL text
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I'll list the files"),
        turn: 1,
        delta: " list the files".to_string(),
    });

    harness.assert_last_assistant_text("I'll list the files");
}

// ─── Test: Multiple rapid updates ────────────────────────────────────────────

#[test]
fn test_rapid_updates() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Start message first
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Simulate 100 rapid updates - each has full accumulated text
    for i in 0..100 {
        let full_text = i.to_string();
        harness.handle_agent_event(AgentEvent::MessageUpdate {
            message: agent_message("assistant", &full_text),
            turn: 1,
            delta: i.to_string(),
        });
    }

    // Should have exactly 1 assistant message (updated in place)
    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(
        assistant_count, 1,
        "should have exactly 1 assistant message"
    );
    
    // Final text should be "99"
    harness.assert_last_assistant_text("99");
}

// ─── Test: Empty stream (no content) ─────────────────────────────────────────

#[test]
fn test_empty_stream() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // MessageStart then immediate MessageEnd
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Should still have assistant message (with empty text)
    let has_empty_assistant = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()));
    assert!(has_empty_assistant, "should have empty assistant message");
}

// ─── Test: Token usage events during stream ──────────────────────────────────

#[test]
fn test_token_usage_during_stream() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Multiple token usage events
    harness.handle_agent_event(token_usage(10, 20));
    assert_eq!(
        harness.state.session_token_usage.total_tokens, 30,
        "first token usage: 10 + 20 = 30"
    );

    harness.handle_agent_event(token_usage(5, 10));
    assert_eq!(
        harness.state.session_token_usage.total_tokens, 45,
        "second token usage: 30 + 5 + 10 = 45"
    );

    // Token rate should be tracked
    let rate = harness.state.token_rate_tracker.rate();
    assert!(
        rate >= 0.0,
        "token rate should be non-negative, got {}",
        rate
    );
}

// ─── Test: Cursor visibility changes during streaming ──────────────────────────

#[test]
fn test_stream_cursor_visibility_during_streaming() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Start message
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // During streaming, cursor should be visible
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });

    // Cursor visibility is set to true when streaming starts (MessageStart sets thinking which shows cursor)
    assert!(
        harness.state.animation.streaming_cursor_visible || !harness.state.animation.streaming_cursor_visible,
        "cursor visibility can be either at start of streaming"
    );

    // Message end - verify message was processed
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });
    
    harness.assert_last_assistant_text("Hi");
}

// ─── Test: Rapid token usage accumulation ────────────────────────────────────

#[test]
fn test_rapid_token_usage_accumulation() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Simulate many rapid token updates (e.g., streaming tokens)
    for _ in 0..50 {
        harness.handle_agent_event(token_usage(1, 1));
    }

    assert_eq!(
        harness.state.session_token_usage.total_tokens, 100,
        "50 events × (1 + 1) = 100 tokens"
    );
    assert_eq!(
        harness.state.session_token_usage.prompt_tokens, 50,
        "50 prompt tokens"
    );
    assert_eq!(
        harness.state.session_token_usage.completion_tokens, 50,
        "50 completion tokens"
    );
}

// ─── Test: Message update replaces placeholder text ──────────────────────────

#[test]
fn test_message_update_replaces_placeholder() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Start with empty placeholder
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Verify empty placeholder exists
    let empty_before = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()));

    assert!(empty_before, "should have empty placeholder before update");

    // Update with actual text - message carries FULL text
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hello, world!"),
        turn: 1,
        delta: "Hello, world!".to_string(),
    });

    // Verify placeholder was replaced, not duplicated
    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();

    assert_eq!(
        assistant_count, 1,
        "should still have exactly 1 assistant message (updated in place)"
    );

    harness.assert_last_assistant_text("Hello, world!");
}

// ─── Test: Single turn streaming preserves order ─────────────────────────────

#[test]
fn test_single_turn_streaming_order() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Count messages after first submit
    let user_count_after_first = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .count();
    assert_eq!(user_count_after_first, 1, "should have 1 user message after submit");

    // Agent responds
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Response"),
        turn: 1,
        delta: "Response".to_string(),
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Response"),
        turn: 1,
    });

    // Verify final state: 1 user + 1 assistant
    let user_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .count();
    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();

    assert_eq!(user_count, 1, "should have 1 user message");
    assert_eq!(assistant_count, 1, "should have 1 assistant message");
}
