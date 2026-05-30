//! Integration tests for full agent conversation flows.
//!
//! Tests verify complete conversation scenarios:
//! - Multi-turn conversations with proper turn separators
//! - Tool execution and results display
//! - Error handling and recovery
//! - Permission request flow
//!
//! Note: Some events (PermissionGranted, PermissionDenied, ContextCompacted) are
//! handled by the agent loop externally and are classified as Ignored by the TUI.

use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

/// Create an AgentMessage with the given role and text content
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

/// Create a default TokenUsage
fn default_token_usage() -> TokenUsage {
    TokenUsage {
        input: 100,
        output: 50,
        cache_read: 0,
        cache_write: 0,
        total_tokens: 150,
    }
}

// ─── Test: Full conversation flow ─────────────────────────────────────────────

#[test]
fn test_full_conversation() {
    let mut harness = AgentTestHarness::new();

    // User: "Hello"
    harness.submit_user_message("Hello");
    assert_eq!(
        harness.state.messages.len(),
        1,
        "user message only after submit"
    );

    // Agent: thinks and responds
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    assert_eq!(
        harness.state.messages.len(),
        2,
        "user + assistant placeholder after MessageStart"
    );

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });

    // Turn ends - no separator without agent_start_time set
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: default_token_usage(),
    });

    // AgentEnd clears agent_running to allow next turn
    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: default_token_usage(),
    });

    // Without agent_start_time, no separator is added
    assert_eq!(
        harness.state.messages.len(),
        2,
        "no separator without agent_start_time"
    );
    assert!(
        !harness.state.agent_running,
        "agent_running cleared after AgentEnd"
    );

    // User: "How are you?"
    harness.submit_user_message("How are you?");
    assert_eq!(
        harness.state.messages.len(),
        3,
        "+ user message"
    );

    // Agent responds
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 2,
    });
    assert_eq!(
        harness.state.messages.len(),
        4,
        "+ assistant placeholder"
    );

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "I'm good"),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 2,
        message_count: 2,
        tool_results_count: 0,
        token_usage: default_token_usage(),
    });

    // Verify conversation history
    let roles: Vec<&str> = harness
        .state
        .messages
        .iter()
        .map(|m| match m {
            MessageItem::User { .. } => "user",
            MessageItem::Assistant { .. } => "assistant",
            MessageItem::Separator { .. } => "separator",
            _ => "other",
        })
        .collect();

    assert_eq!(
        roles,
        vec![
            "user", "assistant",
            "user", "assistant",
        ],
        "conversation should have correct message sequence"
    );
}

// ─── Test: Full conversation with separator (agent_start_time set) ─────────────

#[test]
fn test_full_conversation_with_separator() {
    let mut harness = AgentTestHarness::new();

    // Set agent_start_time to simulate agent being spawned
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });

    // Turn ends with agent_start_time set - separator should be added
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: default_token_usage(),
    });

    // Verify separator was added
    let separators: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(separators.len(), 1, "should have 1 separator");

    // Second turn with separator
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.submit_user_message("How are you?");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "I'm good"),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 2,
        message_count: 2,
        tool_results_count: 0,
        token_usage: default_token_usage(),
    });

    let separators: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(separators.len(), 2, "should have 2 separators");
}

// ─── Test: Agent with tool use ────────────────────────────────────────────────

#[test]
fn test_agent_with_tool_use() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("List files");

    // Agent decides to use tool
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        result: ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "file1.txt\nfile2.rs".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 1,
    });

    // Agent responds with tool results
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Here are the files"),
        turn: 1,
        delta: "Here are the files".to_string(),
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Here are the files"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 1,
        token_usage: default_token_usage(),
    });

    // Verify structure
    assert!(
        harness
            .state
            .messages
            .iter()
            .any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "should have a tool call in messages"
    );
    assert!(
        harness
            .state
            .messages
            .iter()
            .any(|m| matches!(m, MessageItem::Separator { .. })),
        "should have a separator after turn"
    );
}

// ─── Test: Agent error mid-conversation ───────────────────────────────────────

#[test]
fn test_error_mid_conversation() {
    let mut harness = AgentTestHarness::new();

    // First turn succeeds
    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: default_token_usage(),
    });

    // Verify first turn completed - TurnEnd does NOT clear agent_running
    assert!(
        harness.state.agent_running,
        "agent_running stays true after TurnEnd"
    );

    // Second turn fails
    harness.submit_user_message("Cause error");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    assert!(
        !harness.state.agent_running,
        "agent should not be running after error"
    );
    assert!(
        harness
            .state
            .messages
            .iter()
            .any(|m| matches!(m, MessageItem::Error { .. })),
        "should have an error message item"
    );
    assert!(
        matches!(harness.state.mode, crate::tui::state::TuiMode::Chat),
        "should be in Chat mode after error (not stuck in Permission)"
    );
}

// ─── Test: Permission request flow ───────────────────────────────────────────

#[test]
fn test_permission_flow() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Run dangerous command");
    harness.handle_agent_event(AgentEvent::PermissionRequest {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "rm -rf /".to_string(),
        tool_description: "Delete everything".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    });

    assert!(
        matches!(harness.state.mode, crate::tui::state::TuiMode::Permission),
        "should be in Permission mode after permission request"
    );

    // Note: PermissionGranted/PermissionDenied events are sent by the agent loop
    // after user confirms/denies in the UI. The TUI receives these events but
    // they are classified as Ignored in categorize_event - the actual mode
    // transition happens via handle_permission_msg -> handle_permission.
    // This test verifies the PermissionRequest sets the mode correctly.
    assert!(
        harness.state.permission_modal.tool.is_some(),
        "permission modal should have tool set"
    );
}

// ─── Test: Multiple tool calls in one turn ───────────────────────────────────

#[test]
fn test_multiple_tools_in_one_turn() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("List files and check git status");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // First tool
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        result: ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "file1.txt\nfile2.rs".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 1,
    });

    // Second tool
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "git status".to_string(),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "git status".to_string(),
        result: ToolResult {
            tool_call_id: "t2".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "On branch main".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 1,
    });

    // Agent responds
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "I found 2 files and you're on main branch"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 2,
        token_usage: default_token_usage(),
    });

    // Verify both tools are in messages
    let tool_calls: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();
    assert_eq!(tool_calls.len(), 2, "should have exactly 2 tool calls");
}

// ─── Test: Token usage accumulation across turns ─────────────────────────────

#[test]
fn test_token_usage_across_turns() {
    let mut harness = AgentTestHarness::new();

    // First turn
    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TokenUsage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        context_window: 128_000,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 15,
        },
    });

    // Second turn
    harness.submit_user_message("How are you?");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "I'm good"),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::TokenUsage {
        prompt_tokens: 15,
        completion_tokens: 10,
        total_tokens: 25,
        context_window: 128_000,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 2,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 15,
            output: 10,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 25,
        },
    });

    // Verify total token usage accumulated
    assert_eq!(
        harness.state.session_token_usage.total_tokens, 40,
        "total tokens should be accumulated: 15 + 25 = 40"
    );
    assert_eq!(
        harness.state.session_token_usage.prompt_tokens, 25,
        "prompt tokens: 10 + 15 = 25"
    );
    assert_eq!(
        harness.state.session_token_usage.completion_tokens, 15,
        "completion tokens: 5 + 10 = 15"
    );
}

// ─── Test: Non-recoverable error ──────────────────────────────────────────────

#[test]
fn test_non_recoverable_error() {
    let mut harness = AgentTestHarness::new();

    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::Error {
        message: "Internal panic".to_string(),
        error_type: "panic".to_string(),
        recoverable: false,
        context: "".to_string(),
    });

    // Find the error and verify it's marked non-recoverable
    let error_item = harness
        .state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            MessageItem::Error {
                recoverable, ..
            } => Some(recoverable),
            _ => None,
        });
    assert_eq!(
        error_item,
        Some(&false),
        "error should be marked as non-recoverable"
    );
}

// ─── Test: Tool pauses thinking ────────────────────────────────────────────────

#[test]
fn test_tool_pauses_thinking() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("List files");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    assert!(
        harness.state.is_thinking,
        "should be thinking after MessageStart"
    );

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    assert!(
        !harness.state.is_thinking,
        "should NOT be thinking after ToolExecutionStart"
    );
    assert!(
        harness.state.thinking_duration.is_some(),
        "thinking_duration should be recorded when tool starts"
    );
}

// ─── Test: AgentEnd lifecycle ─────────────────────────────────────────────────

#[test]
fn test_agent_end_cleans_up_state() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Done"),
        turn: 1,
    });

    // Set some state that should be cleaned up
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(5));
    harness.state.status_header = Some("Thinking".to_string());
    harness.state.is_thinking = true;

    // Agent ends
    harness.handle_agent_event(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: TokenUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 150,
        },
    });

    // Assert all agent state is cleaned up
    assert!(
        !harness.state.agent_running,
        "agent_running should be false after AgentEnd"
    );
    assert!(
        harness.state.agent_start_time.is_none(),
        "agent_start_time should be None after AgentEnd"
    );
    assert!(
        !harness.state.is_thinking,
        "is_thinking should be false after AgentEnd"
    );
    assert!(
        harness.state.status_header.is_none(),
        "status_header should be None after AgentEnd"
    );
}
