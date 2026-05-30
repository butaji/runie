//! Comprehensive agent event sequence tests following Pi/Codex patterns.
//!
//! Tests verify correct state transitions across the complete agent lifecycle:
//! - User message submission → agent response streaming
//! - Tool execution flows (start → end)
//! - Multi-turn conversations with separators
//! - Error recovery and timeout handling

use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use crate::tui::state::{AppState, Cmd};
use crate::tui::update::system::check_agent_timeout;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};
use std::time::{Duration, Instant};

// ─── Helper Functions ─────────────────────────────────────────────────────────

/// Create a test AgentMessage with the given role and text content
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

/// Create a minimal ToolResult for testing
fn tool_result(content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text {
            text: content.to_string(),
        }],
        is_error,
    }
}

/// Create a TurnEnd event with reasonable defaults
fn turn_end_event(turn: usize) -> AgentEvent {
    AgentEvent::TurnEnd {
        turn,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 150,
        },
    }
}

// ─── Test: Complete happy path - user message → agent response ────────────────

#[test]
fn test_happy_path_user_to_agent_response() {
    let mut harness = AgentTestHarness::new();

    // Arrange: User submits a message
    harness.submit_user_message("Hello");
    harness.assert_agent_not_running(); // Submit doesn't start agent, Cmd::SpawnAgent does
    harness.assert_has_user_message("Hello");

    // Act: Agent starts responding
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.assert_agent_running();
    harness.assert_has_assistant_placeholder();

    // Stream the response incrementally
    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi"),
        turn: 1,
        delta: "Hi".to_string(),
    });
    harness.assert_last_assistant_text("Hi");

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi there"),
        turn: 1,
        delta: " there".to_string(),
    });
    harness.assert_last_assistant_text("Hi there");

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi there!"),
        turn: 1,
    });

    // Assert final state
    harness.assert_last_assistant_text("Hi there!");
    assert!(
        !harness.state.is_thinking,
        "thinking should be false after MessageEnd"
    );
    assert!(
        harness.state.status_header.is_none(),
        "status_header should be cleared after MessageEnd"
    );
}

// ─── Test: Tool execution flow ─────────────────────────────────────────────────

#[test]
fn test_tool_execution_flow() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("List files");

    // Agent starts thinking
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    // Tool execution begins
    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        turn: 1,
    });

    // Verify tool call was added to messages
    let tool_calls: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();
    assert_eq!(
        tool_calls.len(),
        1,
        "should have exactly one tool call after ToolExecutionStart"
    );

    // Check tool call details
    if let MessageItem::ToolCall {
        name,
        args,
        result,
        is_error,
    } = &tool_calls[0]
    {
        assert_eq!(name, "call-1");
        assert_eq!(args, "ls -la");
        assert!(result.is_none(), "result should be None before ToolExecutionEnd");
        assert!(!*is_error, "is_error should be false before ToolExecutionEnd");
    }

    // Tool execution completes successfully
    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls -la".to_string(),
        result: tool_result("total 42\ndrwxr-xr-x  5 admin admin  160 May 29 10:00 .", false),
        duration_ms: 150,
        turn: 1,
    });

    // Verify tool result was updated
    let tool_result_item = harness
        .state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            MessageItem::ToolCall {
                name,
                result: Some(res),
                ..
            } if name == "call-1" => Some(res),
            _ => None,
        });
    assert!(
        tool_result_item.is_some(),
        "tool call should have result after ToolExecutionEnd"
    );
    assert!(
        tool_result_item.unwrap().contains("total 42"),
        "tool result should contain expected output"
    );
}

// ─── Test: Tool execution with error ─────────────────────────────────────────

#[test]
fn test_tool_execution_with_error() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Run command");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_err".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        result: tool_result("Error: command failed with exit code 1", true),
        duration_ms: 50,
        turn: 1,
    });

    // Find the error tool call
    let error_tool = harness
        .state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            MessageItem::ToolCall {
                name,
                is_error: true,
                ..
            } if name == "call_err" => Some(m),
            _ => None,
        });
    assert!(
        error_tool.is_some(),
        "should have an error tool call marked with is_error=true"
    );
}

// ─── Test: Multi-turn conversation ────────────────────────────────────────────

#[test]
fn test_multi_turn_conversation() {
    let mut harness = AgentTestHarness::new();

    // Turn 1: User asks hello, agent responds
    harness.submit_user_message("Hello");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi there!"),
        turn: 1,
    });
    harness.handle_agent_event(turn_end_event(1));

    // Turn 2: User asks follow-up, agent responds
    harness.submit_user_message("How are you?");
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 2,
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "I'm doing well!"),
        turn: 2,
    });
    harness.handle_agent_event(turn_end_event(2));

    // Assert 2 turn separators
    let separators: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(
        separators.len(),
        2,
        "should have exactly 2 turn separators after 2 turns"
    );

    // Assert 2 user messages
    let user_messages: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .collect();
    assert_eq!(
        user_messages.len(),
        2,
        "should have exactly 2 user messages"
    );
}

// ─── Test: Error recovery ──────────────────────────────────────────────────────

#[test]
fn test_error_recovery() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Agent starts
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    // Agent errors
    harness.handle_agent_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    // Assert state cleaned up
    assert!(
        !harness.state.agent_running,
        "agent_running should be false after error"
    );
    assert!(
        harness.state.status_header.is_none(),
        "status_header should be None after error"
    );
    assert!(
        harness.state.is_thinking == false,
        "is_thinking should be false after error"
    );

    // Assert error message was added
    let has_error = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Error { .. }));
    assert!(has_error, "should have an Error message item");
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

// ─── Test: Timeout handling ───────────────────────────────────────────────────

#[test]
fn test_timeout_handling() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // Agent starts running
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    harness.assert_agent_running();

    // Simulate timeout by setting agent_start_time to 601 seconds ago
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(601));

    // Call check_agent_timeout directly
    let cmds = check_agent_timeout(&mut harness.state);

    // Assert timeout was triggered
    assert!(
        !harness.state.agent_running,
        "agent_running should be false after timeout"
    );
    assert!(
        harness.state.agent_start_time.is_none(),
        "agent_start_time should be cleared after timeout"
    );
    assert!(
        harness.state.is_thinking == false,
        "is_thinking should be false after timeout"
    );
    assert!(
        cmds.is_some() && cmds.as_ref().unwrap().contains(&Cmd::Interrupt),
        "should return Cmd::Interrupt after timeout"
    );

    // Assert system message was added about timeout
    let has_timeout_msg = harness
        .state
        .messages
        .iter()
        .any(|m| match m {
            MessageItem::System { text } => text.contains("timed out"),
            _ => false,
        });
    assert!(
        has_timeout_msg,
        "should have a system message about timeout"
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
        harness.state.is_thinking == false,
        "is_thinking should be false after AgentEnd"
    );
    assert!(
        harness.state.status_header.is_none(),
        "status_header should be None after AgentEnd"
    );
}

// ─── Test: MessageStart creates placeholder ───────────────────────────────────

#[test]
fn test_message_start_creates_placeholder_once() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    // First MessageStart
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    let assistant_count_after_first = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(
        assistant_count_after_first, 1,
        "should have exactly 1 assistant message after first MessageStart"
    );

    // Second MessageStart (e.g., if agent restarts mid-turn)
    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    let assistant_count_after_second = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(
        assistant_count_after_second, 1,
        "second MessageStart should NOT add another placeholder (deduped)"
    );
}

// ─── Test: TurnEnd adds separator ──────────────────────────────────────────────

#[test]
fn test_turn_end_adds_separator() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Response"),
        turn: 1,
    });

    // Without agent_start_time set, no separator should be added
    let separators_before: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert!(
        separators_before.is_empty(),
        "no separator without agent_start_time"
    );

    // Set agent_start_time and trigger turn end
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    harness.handle_agent_event(turn_end_event(1));

    let separators_after: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();
    assert_eq!(
        separators_after.len(),
        1,
        "should have 1 separator after TurnEnd with agent_start_time"
    );
}

// ─── Test: Thinking indicator added for long think times ──────────────────────

#[test]
fn test_thinking_indicator_added_for_long_think() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Simulate some thinking time by directly setting thinking_start in the past
    harness.state.thinking_start = Some(Instant::now() - Duration::from_millis(1500));

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Quick response"),
        turn: 1,
    });

    // Check that a Thought item was added (duration > 0.5s)
    let has_thought = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Thought { .. }));
    assert!(
        has_thought,
        "should have Thought item when thinking duration > 0.5s"
    );
}

// ─── Test: Quick think doesn't add indicator ──────────────────────────────────

#[test]
fn test_quick_think_no_indicator() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // No delay - instant response

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi"),
        turn: 1,
    });

    // Check that NO Thought item was added (duration < 0.5s)
    let has_thought = harness
        .state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::Thought { .. }));
    assert!(
        !has_thought,
        "should NOT have Thought item when thinking duration < 0.5s"
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
        tool_call_id: "call-1".to_string(),
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

// ─── Extension traits for test assertions ─────────────────────────────────────

impl AgentTestHarness {
    /// Assert harness has no agent running
    fn assert_agent_not_running(&self) {
        assert!(
            !self.state.agent_running,
            "agent should NOT be running"
        );
    }

    /// Assert harness has a user message containing the given text
    fn assert_has_user_message(&self, text: &str) {
        let has_message = self.state.messages.iter().any(|m| match m {
            MessageItem::User { text: t, .. } => t.contains(text),
            _ => false,
        });
        assert!(
            has_message,
            "should have user message containing: {}",
            text
        );
    }
}
