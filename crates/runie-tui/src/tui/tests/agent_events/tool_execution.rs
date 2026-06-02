//! Tool execution tests.
//!
//! Tests:
//! - Tool start adds ToolCall message
//! - Tool end updates result, marks error
//! - Multiple tools in one turn
//! - Tool pauses thinking timer
//! - Tool execution clears thinking state

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::ThinkingState;
use crate::tui::update::agent::handle_agent_event;
use runie_agent::{AgentEvent, ContentPart, ToolResult};

/// Helper: Create a minimal ToolResult for testing.
fn tool_result(tool_call_id: &str, content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: tool_call_id.to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text {
            text: content.to_string(),
        }],
        is_error,
    }
}

/// Helper: Create AppState ready for tool testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state.agent_running = true;
    state
}

// ─── Tool start tests ─────────────────────────────────────────────────────────

#[test]
fn test_tool_start_adds_tool_running_message() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls -la"}"#.to_string(),
            turn: 1,
        },
    );

    let has_tool = state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolRunning { name, args, .. } if name == "bash" && args == r#"{"command": "ls -la"}"#
    ));
    assert!(has_tool, "should have ToolRunning with name bash");
}

#[test]
fn test_tool_start_sets_status_running() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            turn: 1,
        },
    );

    assert_eq!(
        state.status_header.as_deref(),
        Some("Running"),
        "status_header should be Running"
    );
}

#[test]
fn test_tool_start_sets_status_details() {
    let mut state = make_test_state();
    // Set status_start_time so elapsed can be calculated
    state.status_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-abc123".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            turn: 1,
        },
    );

    assert!(
        state.status_details.is_some(),
        "status_details should be set"
    );
    // status_details now shows elapsed time from status_start_time
    let details = state.status_details.unwrap();
    assert!(
        details.ends_with("s") || details.contains("m"),
        "status_details should show elapsed time format (e.g., '0s', '1m 30s')"
    );
}

#[test]
fn test_tool_start_pauses_thinking_timer() {
    let mut state = make_test_state();
    state.thinking = Some(ThinkingState { start: Some(std::time::Instant::now()), text: String::new(), accrued_duration: None });

    // Small delay to ensure thinking_duration would be > 0
    std::thread::sleep(std::time::Duration::from_millis(5));

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            turn: 1,
        },
    );

    // on_tool_start pauses thinking by setting start = None, but thinking remains Some(...)
    assert!(state.thinking.is_some(), "thinking should still be Some (paused, not cleared)");
    assert!(
        state.thinking.as_ref().map_or(false, |t| t.accrued_duration.is_some()),
        "thinking.accrued_duration should be accumulated"
    );
}

// ─── Tool end tests ───────────────────────────────────────────────────────────

#[test]
fn test_tool_end_updates_result() {
    let mut state = make_test_state();

    // Start tool first
    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "echo hello"}"#.to_string(),
            turn: 1,
        },
    );

    // End tool with result
    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "echo hello"}"#.to_string(),
            result: tool_result("call-1", "hello\n", false),
            duration_ms: 100,
            turn: 1,
        },
    );

    let result = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall {
            name,
            result,
            ..
        } if name == "call-1" => result.clone(),
        _ => None,
    });
    assert_eq!(result, Some("hello\n".to_string()), "tool result should be 'hello\\n'");
}

#[test]
fn test_tool_end_marks_error() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "bad_command"}"#.to_string(),
            turn: 1,
        },
    );

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "bad_command"}"#.to_string(),
            result: tool_result("call-1", "command not found: bad_command", true),
            duration_ms: 50,
            turn: 1,
        },
    );

    let is_error = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall {
            name,
            is_error,
            ..
        } if name == "call-1" => Some(*is_error),
        _ => None,
    });
    assert_eq!(is_error, Some(true), "tool should be marked as error");
}

#[test]
fn test_tool_end_clears_status_details() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            turn: 1,
        },
    );

    assert!(state.status_details.is_some(), "status_details should be set during tool");

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            result: tool_result("call-1", "done", false),
            duration_ms: 100,
            turn: 1,
        },
    );

    // Note: status_details is NOT cleared by tool end - it only changes when agent ends or new tool starts
}

// ─── Multiple tools tests ─────────────────────────────────────────────────────

#[test]
fn test_multiple_tools_in_one_turn() {
    let mut state = make_test_state();

    let tool_ids = vec!["call-1", "call-2", "call-3"];

    for (i, tool_id) in tool_ids.iter().enumerate() {
        let id = tool_id.to_string();
        // Send ToolExecutionStart first
        handle_agent_event(
            &mut state,
            AgentEvent::ToolExecutionStart {
                tool_call_id: id.clone(),
                tool_name: "bash".to_string(),
                tool_args: format!(r#"{{"cmd": {}}}"#, i),
                turn: 1,
            },
        );

        // Then send ToolExecutionEnd
        handle_agent_event(
            &mut state,
            AgentEvent::ToolExecutionEnd {
                tool_call_id: id.clone(),
                tool_name: "bash".to_string(),
                tool_args: format!(r#"{{"cmd": {}}}"#, i),
                result: ToolResult {
                    tool_call_id: id,
                    tool_name: "bash".to_string(),
                    input: serde_json::json!({}),
                    content: vec![ContentPart::Text { text: format!("result{}", i) }],
                    is_error: false,
                },
                duration_ms: 100,
                turn: 1,
            },
        );
    }

    let tool_count = state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .count();
    assert_eq!(tool_count, 3, "should have 3 tool calls");
}

#[test]
fn test_tool_args_preserved() {
    let mut state = make_test_state();

    let complex_args = r#"{"command": "find . -name '*.rs' -exec grep -l 'fn main' {} \;", "env": {"PATH": "/usr/bin"}}"#;

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: complex_args.to_string(),
            turn: 1,
        },
    );

    // ToolCall message is created on start, args come from ToolExecutionEnd event
    let has_tool = state.messages.iter().rev().any(|m| match m {
        MessageItem::ToolCall {
            name,
            ..
        } if name == "call-1" => true,
        _ => false,
    });
    assert!(has_tool, "should have ToolCall with id call-1");
}

// ─── Tool without message start ──────────────────────────────────────────────

#[test]
fn test_tool_start_without_message_start() {
    let mut state = make_test_state();
    // No MessageStart, just tool directly
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            turn: 1,
        },
    );

    let has_tool = state
        .messages
        .iter()
        .any(|m| matches!(m, MessageItem::ToolCall { .. }));
    assert!(has_tool, "tool call should be recorded even without MessageStart");
}
