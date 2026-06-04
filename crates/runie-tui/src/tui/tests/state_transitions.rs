//! Table-driven tests for agent state transitions.
//!
//! Tests state changes in response to AgentEvent variants:
//! - Message lifecycle (start, update, end)
//! - Tool execution lifecycle (start, end)
//! - Agent lifecycle (agent end)
//! - Error handling
//! - Token usage accumulation

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::ThinkingState;
use crate::tui::update::agent;
use runie_agent::{AgentEvent, AgentMessage, ContentPart::Text, TokenUsage as AgentTokenUsage};
use std::time::Instant;

/// Helper: Create an AgentMessage with given role and content text.
fn agent_message(role: &str, content: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![Text { text: content.to_string() }],
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

/// Test: message start sets thinking
#[test]
fn test_message_start_sets_thinking() {
    let mut state = AppState::default();
    state.agent_running = true;
    agent::handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(state.thinking.is_some(), "thinking should be Some");
    assert_eq!(state.status_header, Some("Thinking".to_string()), "status_header");
}

/// Test: tool start pauses thinking
#[test]
fn test_tool_start_pauses_thinking() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.thinking = Some(ThinkingState { start: Some(Instant::now()), text: String::new(), accrued_duration: None });
    agent::handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        },
    );
    assert!(state.thinking.is_none(), "thinking should be None");
    assert!(state.thinking.as_ref().map_or(false, |t| t.accrued_duration.is_some()), "thinking.accrued_duration should be set");
    assert_eq!(state.status_header, Some("Running".to_string()), "status_header");
}

/// Test: agent end clears all
#[test]
fn test_agent_end_clears_all() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.thinking = Some(ThinkingState { start: Some(Instant::now()), text: String::new(), accrued_duration: None });
    state.status_header = Some("Thinking".to_string());
    state.status_start_time = Some(Instant::now());
    agent::handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: AgentTokenUsage::default(),
        },
    );
    assert!(!state.agent_running, "agent_running should be false");
    assert!(state.thinking.is_none(), "thinking should be None");
    assert!(state.thinking.as_ref().map_or(true, |t| t.start.is_none()), "thinking.start should be none");
    assert!(state.status_header.is_none(), "status_header should be none");
    assert!(state.status_start_time.is_none(), "status_start_time should be none");
}

/// Test: error clears running but leaves message
#[test]
fn test_error_clears_running_but_leaves_message() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.messages.push(MessageItem::Assistant {
        text: "".to_string(),
        model: None,
        timestamp: None,
    });
    agent::handle_agent_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );
    assert!(!state.agent_running, "agent_running should be false");
    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "should have error message"
    );
}

/// Test: single token usage event
#[test]
fn test_token_usage_single_event() {
    let mut state = AppState::default();
    agent::handle_agent_event(&mut state, token_usage(10, 20));
    assert_eq!(state.session_token_usage.prompt_tokens, 10);
    assert_eq!(state.session_token_usage.completion_tokens, 20);
    assert_eq!(state.session_token_usage.total_tokens, 30);
}

/// Test: multiple token usage events accumulate
#[test]
fn test_token_usage_multiple_events() {
    let mut state = AppState::default();
    agent::handle_agent_event(&mut state, token_usage(10, 20));
    agent::handle_agent_event(&mut state, token_usage(5, 10));
    assert_eq!(state.session_token_usage.prompt_tokens, 15);
    assert_eq!(state.session_token_usage.completion_tokens, 30);
    assert_eq!(state.session_token_usage.total_tokens, 45);
}

/// Test: zero tokens
#[test]
fn test_token_usage_zero() {
    let mut state = AppState::default();
    agent::handle_agent_event(&mut state, token_usage(0, 0));
    assert_eq!(state.session_token_usage.prompt_tokens, 0);
    assert_eq!(state.session_token_usage.completion_tokens, 0);
    assert_eq!(state.session_token_usage.total_tokens, 0);
}

/// Test: message start adds placeholder
#[test]
fn test_message_start_adds_placeholder() {
    let mut state = AppState::default();
    state.agent_running = true;
    agent::handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "should have empty assistant placeholder"
    );
}

/// Test: message update fills placeholder
#[test]
fn test_message_update_fills_placeholder() {
    let mut state = AppState::default();
    agent::handle_agent_event(
        &mut state,
        AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 1,
        },
    );
    agent::handle_agent_event(
        &mut state,
        AgentEvent::MessageUpdate {
            message: agent_message("assistant", "Hello"),
        delta: "Hello".to_string(),
        replace: false,
        turn: 1,
        },
    );
    let has_hello = state.messages.iter().any(|m| matches!(
        m,
        MessageItem::Assistant { text, .. } if text.contains("Hello")
    ));
    assert!(has_hello, "should have Hello in messages");
}

/// Test: tool start adds tool call message
#[test]
fn test_tool_start_adds_tool_call_message() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.thinking = Some(ThinkingState { start: Some(Instant::now()), text: String::new(), accrued_duration: None });
    agent::handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "tool_1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls -la".to_string(),
            turn: 1,
        },
    );
    let has_tool_call = state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. }));
    assert!(has_tool_call, "tool_call should be present");
    assert_eq!(state.status_header.as_ref().map(|s| s.as_str()), Some("Running"));
}

/// Test: tool end updates tool call result
#[test]
fn test_tool_end_updates_tool_call_result() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.messages.push(MessageItem::ToolCall {
        name: "tool_1".to_string(),
        args: "ls".to_string(),
        result: None,
        is_error: false,
    });
    agent::handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool_1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            result: runie_agent::events::ToolResult {
                tool_call_id: "tool_1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![Text { text: "file1.txt\nfile2.rs".to_string() }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        },
    );
    let has_tool_call = state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. }));
    assert!(has_tool_call, "tool_call should be present");
    assert_eq!(state.status_header.as_ref().map(|s| s.as_str()), None);
}
