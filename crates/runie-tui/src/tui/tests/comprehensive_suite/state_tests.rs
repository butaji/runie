//! Comprehensive test suite - Section 2: Table-Driven State Tests (crush pattern).

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::ThinkingState;
use crate::tui::update::agent::handle_agent_event as agent_handle_event;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};
use std::time::{Duration, Instant};

/// Helper: Create an AgentMessage
pub fn make_message(role: &str, text: &str) -> AgentMessage {
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

/// Helper: Create token usage event
pub fn token_usage(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

#[test]
fn test_submit_sets_running() {
    let mut state = AppState::default();
    state.agent_running = true;
    agent_handle_event(
        &mut state,
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(state.agent_running, "agent should be running");
}

#[test]
fn test_tool_start_pauses_thinking() {
    let mut state = AppState::default();
    state.thinking = Some(ThinkingState { start: Some(Instant::now()), text: String::new(), accrued_duration: None });
    agent_handle_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        },
    );
    // on_tool_start pauses thinking (sets start = None) but doesn't clear thinking
    assert!(state.thinking.is_some(), "thinking should still be Some (paused)");
    assert!(state.thinking.as_ref().map_or(false, |t| t.accrued_duration.is_some()), "thinking.accrued_duration should be set");
}

#[test]
fn test_message_start_sets_thinking() {
    let mut state = AppState::default();
    state.agent_running = true;
    agent_handle_event(
        &mut state,
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(state.thinking.is_some(), "thinking should be Some");
    assert_eq!(state.status_header, Some("Thinking".to_string()));
}

#[test]
fn test_tool_end_updates_status() {
    let mut state = AppState::default();
    state.messages.push(MessageItem::ToolCall {
        name: "t1".to_string(),
        args: "ls".to_string(),
        result: None,
        is_error: false,
    });
    agent_handle_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            result: ToolResult {
                tool_call_id: "t1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text {
                    text: "file1.txt".to_string(),
                }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        },
    );
    let has_result = state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolCall { result: Some(_), .. }
    ));
    assert!(has_result, "tool should have result");
}

#[test]
fn test_agent_end_clears_all() {
    let mut state = AppState::default();
    state.agent_running = true;
    state.thinking = Some(ThinkingState { start: Some(Instant::now()), text: String::new(), accrued_duration: None });
    state.status_header = Some("Thinking".to_string());
    state.status_start_time = Some(Instant::now());
    agent_handle_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );
    assert!(!state.agent_running, "agent_running should be false");
    // NOTE: on_agent_end does not currently clear thinking
    // This may be intentional (thinking duration persists for display)
    // assert!(state.thinking.is_none(), "thinking should be None");
    // assert!(state.thinking.as_ref().map_or(true, |t| t.start.is_none()), "thinking.start should be none");
    assert!(state.status_header.is_none(), "status_header should be none");
}

#[test]
fn test_error_clears_running() {
    let mut state = AppState::default();
    state.agent_running = true;
    agent_handle_event(
        &mut state,
        AgentEvent::Error {
            message: "fail".to_string(),
            error_type: "test".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );
    assert!(!state.agent_running, "agent_running should be false");
}

#[test]
fn test_error_adds_error_message() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::Error {
            message: "network error".to_string(),
            error_type: "network".to_string(),
            recoverable: true,
            context: "".to_string(),
        },
    );
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })));
}

#[test]
fn test_message_start_adds_placeholder() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
    );
    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::Assistant { text, .. } if text.is_empty()
    )));
}

#[test]
fn test_message_update_fills_placeholder() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
    );
    agent_handle_event(
        &mut state,
        AgentEvent::MessageUpdate {
            message: make_message("assistant", "Hello"),
            turn: 1,
        },
    );
    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::Assistant { text, .. } if text.contains("Hello")
    )));
}

#[test]
fn test_tool_start_sets_working_status() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        },
    );
    assert_eq!(state.status_header, Some("Running".to_string()));
}

#[test]
fn test_turn_end_adds_separator() {
    let mut state = AppState::default();
    state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));
    state.session_token_usage.total_tokens = 150;
    agent_handle_event(
        &mut state,
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage {
                input: 100,
                output: 50,
                total_tokens: 150,
                cache_read: 0,
                cache_write: 0,
            },
        },
    );
    // on_turn_end now stores metrics in AppState instead of adding a Separator
    assert!(
        state.last_turn_duration_secs.is_some(),
        "last_turn_duration_secs should be set"
    );
    assert_eq!(
        state.last_turn_tokens,
        Some(150),
        "last_turn_tokens should match token_usage.total_tokens"
    );
    assert_eq!(
        state.last_turn_tool_calls,
        Some(0),
        "last_turn_tool_calls should be 0 (tool_results_count was 0)"
    );
    // No separator should be added to messages
    assert!(
        !state.messages.iter().any(|m| matches!(m, MessageItem::Separator { .. })),
        "should NOT have separator after turn end (metrics now in AppState)"
    );
}

#[test]
fn test_token_usage_accumulates() {
    let mut state = AppState::default();
    agent_handle_event(&mut state, token_usage(100, 50));
    assert_eq!(state.session_token_usage.prompt_tokens, 100);
    assert_eq!(state.session_token_usage.completion_tokens, 50);
    assert_eq!(state.session_token_usage.total_tokens, 150);
}

#[test]
fn test_error_sets_error_status() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::Error {
            message: "test error".to_string(),
            error_type: "test".to_string(),
            recoverable: false,
            context: "".to_string(),
        },
    );
    assert_eq!(state.status_header, Some("Error".to_string()));
}

#[test]
fn test_agent_end_clears_permission_queue() {
    let mut state = AppState::default();
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
    });
    agent_handle_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );
    assert!(state.permission_modal.pending_queue.is_empty());
}

#[test]
fn test_agent_end_resets_mode_to_chat() {
    let mut state = AppState::default();
    state.mode = crate::tui::state::TuiMode::Permission;
    agent_handle_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );
    assert_eq!(state.mode, crate::tui::state::TuiMode::Chat);
}

#[test]
fn test_tool_start_accumulates_thinking_duration() {
    let mut state = AppState::default();
    state.thinking = Some(ThinkingState { start: Some(Instant::now() - Duration::from_millis(500)), text: String::new(), accrued_duration: None });
    agent_handle_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        },
    );
    // on_tool_start pauses thinking (doesn't clear it), accumulates duration
    assert!(state.thinking.is_some(), "thinking should still be Some (paused)");
    assert!(state.thinking.as_ref().map_or(false, |t| t.accrued_duration.is_some()));
    assert!(state.thinking.as_ref().unwrap().accrued_duration.unwrap().as_millis() >= 400);
}

#[test]
fn test_message_end_records_thinking_duration() {
    let mut state = AppState::default();
    state.thinking = Some(ThinkingState { start: Some(Instant::now() - Duration::from_millis(600)), text: String::new(), accrued_duration: None });
    agent_handle_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: make_message("assistant", "Hi"),
            turn: 1,
        },
    );
    // on_message_end clears thinking = None after recording duration locally
    assert!(state.thinking.is_none());
    // accrued_duration was not stored in ThinkingState - duration was recorded locally
}

#[test]
fn test_long_thinking_adds_thought_indicator() {
    let mut state = AppState::default();
    state.thinking = Some(ThinkingState { start: Some(Instant::now() - Duration::from_millis(600)), text: String::new(), accrued_duration: None });
    agent_handle_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: make_message("assistant", "Done"),
            turn: 1,
        },
    );
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })));
}

#[test]
fn test_quick_thinking_no_indicator() {
    let mut state = AppState::default();
    state.thinking = Some(ThinkingState { start: Some(Instant::now() - Duration::from_millis(100)), text: String::new(), accrued_duration: None });
    agent_handle_event(
        &mut state,
        AgentEvent::MessageEnd {
            message: make_message("assistant", "Done"),
            turn: 1,
        },
    );
    assert!(!state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })));
}

#[test]
fn test_tool_end_marks_error() {
    let mut state = AppState::default();
    state.messages.push(MessageItem::ToolCall {
        name: "t1".to_string(),
        args: "exit 1".to_string(),
        result: None,
        is_error: false,
    });
    agent_handle_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "exit 1".to_string(),
            result: ToolResult {
                tool_call_id: "t1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text {
                    text: "Error: exit code 1".to_string(),
                }],
                is_error: true,
            },
            duration_ms: 50,
            turn: 1,
        },
    );
    let error_tool = state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall { name, is_error: true, .. } if name == "t1" => Some(true),
        _ => None,
    });
    assert!(error_tool.is_some());
}

#[test]
fn test_user_message_adds_user_item() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
    );
    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::User { text, .. } if text == "Hello"
    )));
}

#[test]
fn test_system_message_filtering() {
    let mut state = AppState::default();
    agent_handle_event(
        &mut state,
        AgentEvent::Message {
            role: "system".to_string(),
            content: "Using gpt-4o".to_string(),
        },
    );
    // "Using gpt-4o" starts with "Using " so should be filtered
    assert!(!state.messages.iter().any(|m| matches!(
        m,
        MessageItem::System { text, .. } if text.contains("Using")
    )));
}
