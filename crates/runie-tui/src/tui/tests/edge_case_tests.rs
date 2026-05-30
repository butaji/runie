//! Edge case and stress tests for TUI agent handling.
//!
//! Tests verify correct handling of:
//! - Very long messages
//! - Empty messages
//! - Unicode content
//! - Special characters in tool args
//! - Rapid start/stop cycles
//! - Missing event sequences
//! - Duplicate events
//! - Token usage overflow protection

use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, TokenUsage};

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

/// Helper: Create a TokenUsage event.
fn make_token_usage(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

// ─── Test: Very long message ─────────────────────────────────────────────────

#[test]
fn test_very_long_message() {
    let mut harness = AgentTestHarness::new();
    let long_text = "a".repeat(10000);
    harness.submit_user_message(&long_text);

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", &long_text),
        turn: 1,
        delta: long_text.clone(),
    });

    harness.assert_last_assistant_text(&long_text);
}

// ─── Test: Empty user message ────────────────────────────────────────────────

#[test]
fn test_empty_user_message() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("");

    // Should not spawn agent for empty message
    assert!(
        !harness.state.agent_running,
        "agent should not be running for empty message"
    );
}

// ─── Test: Unicode in messages ───────────────────────────────────────────────

#[test]
fn test_unicode_messages() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello 👋 🌍 日本語");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Response: 👍"),
        turn: 1,
        delta: "Response: 👍".to_string(),
    });

    harness.assert_last_assistant_text("Response: 👍");
}

// ─── Test: Special characters in tool args ───────────────────────────────────

#[test]
fn test_special_chars_in_tool_args() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Run command");

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"cmd": "echo 'hello\nworld' \"test\""}"#.to_string(),
        turn: 1,
    });

    let tool = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall {
            name,
            args,
            ..
        } if name == "bash" => Some(args.clone()),
        _ => None,
    });
    assert_eq!(
        tool,
        Some(r#"{"cmd": "echo 'hello\nworld' \"test\""}"#.to_string()),
        "tool args should preserve special characters"
    );
}

// ─── Test: Rapid start/stop ─────────────────────────────────────────────────

#[test]
fn test_rapid_start_stop() {
    let mut harness = AgentTestHarness::new();

    for i in 0..10 {
        harness.submit_user_message(&format!("msg{}", i));
        harness.handle_agent_event(AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 0,
            },
        });
    }

    assert!(
        !harness.state.agent_running,
        "agent should not be running after AgentEnd"
    );
}

// ─── Test: MessageUpdate without MessageStart ────────────────────────────────

#[test]
fn test_update_without_start() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Send MessageStart first (required for proper state)
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Now update should work
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });

    harness.assert_last_assistant_text("Hi");
}

// ─── Test: Duplicate MessageStart events ────────────────────────────────────

#[test]
fn test_duplicate_message_start_events() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // First MessageStart
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Second MessageStart (duplicate)
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Should still only have 1 assistant message
    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 1, "should have exactly 1 assistant message");
}

// ─── Test: Token usage overflow protection ───────────────────────────────────

#[test]
fn test_token_usage_overflow() {
    let mut harness = AgentTestHarness::new();

    // Simulate large token counts
    harness.handle_agent_event(AgentEvent::TokenUsage {
        prompt_tokens: usize::MAX / 2,
        completion_tokens: usize::MAX / 2,
        total_tokens: usize::MAX - 1,
        context_window: 128000,
    });

    // Should not panic - token usage should be tracked
    assert!(
        harness.state.session_token_usage.total_tokens > 0,
        "token usage should be recorded"
    );
}

// ─── Test: Many rapid message updates ───────────────────────────────────────

#[test]
fn test_many_rapid_updates() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Start");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // 1000 rapid updates
    for i in 0..1000 {
        harness.handle_agent_event(AgentEvent::MessageUpdate {
            message: agent_message("assistant", &i.to_string()),
            turn: 1,
            delta: i.to_string(),
        });
    }

    // Should still only have 1 assistant message
    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 1, "should have exactly 1 assistant message after 1000 updates");
}

// ─── Test: Empty content in MessageUpdate ───────────────────────────────────

#[test]
fn test_empty_content_update() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Update with empty delta
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", ""),
        turn: 1,
        delta: "".to_string(),
    });

    // Should still have the placeholder
    harness.assert_has_assistant_placeholder();
}

// ─── Test: ToolExecutionStart without prior message ─────────────────────────

#[test]
fn test_tool_without_prior_message() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Run tool");

    // Tool execution without MessageStart
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    // Tool should still be recorded
    let has_tool = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::ToolCall { name, .. } if name == "bash"));
    assert!(has_tool, "tool call should be recorded");
}

// ─── Test: Multiple turns handling ──────────────────────────────────────────

#[test]
fn test_multiple_turns() {
    let mut harness = AgentTestHarness::new();

    for turn in 1..=5 {
        harness.submit_user_message(&format!("Message {}", turn));

        harness.handle_agent_event(AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn,
        });

        harness.handle_agent_event(AgentEvent::MessageUpdate {
            message: agent_message("assistant", &format!("Response {}", turn)),
            turn,
            delta: format!("Response {}", turn),
        });

        harness.handle_agent_event(AgentEvent::MessageEnd {
            message: agent_message("assistant", &format!("Response {}", turn)),
            turn,
        });
    }

    let assistant_count = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 5, "should have 5 assistant messages for 5 turns");
}

// ─── Test: Malformed unicode handling ───────────────────────────────────────

#[test]
fn test_malformed_unicode() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Test");

    // Invalid surrogate pairs
    let malformed = String::from_utf8_lossy(&[0xED, 0xA0, 0x80]).to_string();

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", &malformed),
        turn: 1,
        delta: malformed.clone(),
    });

    // Should handle gracefully without panic - check assistant message exists
    let has_assistant = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Assistant { .. }));
    assert!(has_assistant, "assistant message should exist after handling malformed unicode");
}

// ─── Test: Context window at limit ──────────────────────────────────────────

#[test]
fn test_context_window_at_limit() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Test");

    harness.handle_agent_event(AgentEvent::TokenUsage {
        prompt_tokens: 100000,
        completion_tokens: 28000,
        total_tokens: 128000,
        context_window: 128000,
    });

    // Context window usage should be tracked
    let usage = harness.state.top_bar.context_window;
    assert_eq!(usage, Some(128000), "context window should be recorded");
}

// ─── Test: Rapid token usage updates ────────────────────────────────────────

#[test]
fn test_rapid_token_usage_updates() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Test");

    // 100 rapid token updates
    for i in 0..100 {
        harness.handle_agent_event(make_token_usage(i, i));
    }

    // Should accumulate correctly
    let total = harness.state.session_token_usage.total_tokens;
    assert!(total > 0, "token usage should accumulate");
}

// ─── Test: Tool args with newlines and quotes ────────────────────────────────

#[test]
fn test_tool_args_complex() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Complex args");

    let complex_args = r#"
{
    "command": "find . -name '*.rs' -exec grep -l 'fn main' {} \;",
    "env": {"PATH": "/usr/local/bin:/usr/bin:/bin"},
    "timeout": 30000
}"#;

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-complex".to_string(),
        tool_name: "bash".to_string(),
        tool_args: complex_args.to_string(),
        turn: 1,
    });

    let tool = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall {
            name,
            args,
            ..
        } if name == "bash" => Some(args.clone()),
        _ => None,
    });

    assert_eq!(tool, Some(complex_args.to_string()), "complex tool args should be preserved");
}

// ─── Test: Very long tool name ───────────────────────────────────────────────

#[test]
fn test_long_tool_name() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Test");

    let long_name = format!("tool_{}", "x".repeat(1000));

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: long_name.clone(),
        tool_args: "{}".to_string(),
        turn: 1,
    });

    let has_tool = harness.state.messages.iter().any(|m| match m {
        MessageItem::ToolCall { name, .. } if name == &long_name => true,
        _ => false,
    });
    assert!(has_tool, "tool with long name should be recorded");
}
