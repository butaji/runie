//! Comprehensive tool execution tests.
//!
//! Tests:
//! - Bash tool execution
//! - Tool error handling
//! - Multiple tools in one turn
//! - Edit tool operation
//! - Tool result with JSON content
//! - Tool execution clears thinking state

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::agent::handle_agent_event;
use runie_agent::{AgentEvent, ContentPart, ToolResult};

/// Helper: Create a minimal ToolResult for testing
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

/// Helper: Create AppState ready for testing
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state.agent_running = true;
    state
}

/// Helper: execute a bash tool and return the state
fn execute_bash_tool(args: &str) -> AppState {
    let mut state = make_test_state();
    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: args.to_string(),
            turn: 1,
        },
    );
    state
}

/// Helper: complete a bash tool execution
fn complete_bash_tool(state: &mut AppState, result_text: &str) {
    handle_agent_event(
        state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "echo hello"}"#.to_string(),
            result: ToolResult {
                tool_call_id: "call-1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text {
                    text: result_text.to_string(),
                }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        },
    );
}

/// Test: Bash tool start adds tool call
#[test]
fn test_bash_tool_start() {
    let mut state = execute_bash_tool(r#"{"command": "echo hello"}"#);

    let tool = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, args, .. } if name == "bash" => Some(args.clone()),
        _ => None,
    });
    assert!(tool.is_some());
    assert_eq!(tool.unwrap(), r#"{"command": "echo hello"}"#);
}

/// Test: Bash tool end updates result
#[test]
fn test_bash_tool_end() {
    let mut state = execute_bash_tool(r#"{"command": "echo hello"}"#);
    complete_bash_tool(&mut state, "hello\n");

    let tool_result_text = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, result, .. } if name == "bash" => result.clone(),
        _ => None,
    });
    assert_eq!(tool_result_text, Some("hello\n".to_string()), "tool result should be hello\\n");
}

/// Test: Tool error handling
#[test]
fn test_tool_error() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "bad_command"}"#.to_string(),
        turn: 1,
    });

    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "bad_command"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text { text: "command not found".to_string() }],
            is_error: true,
        },
        duration_ms: 50,
        turn: 1,
    });

    let is_error = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, is_error, .. } if name == "bash" => Some(*is_error),
        _ => None,
    });
    assert_eq!(is_error.as_ref(), Some(&true), "tool should be marked as error");
}

/// Test: Multiple tools in one turn
#[test]
fn test_multiple_tools() {
    let mut state = make_test_state();

    for i in 0..3 {
        let tool_call_id = format!("call-{}", i);
        let tool_args = format!(r#"{{"cmd": {}}}"#, i);

        handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
            tool_call_id: tool_call_id.clone(),
            tool_name: "bash".to_string(),
            tool_args: tool_args.clone(),
            turn: 1,
        });

        handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
            tool_call_id: tool_call_id.clone(),
            tool_name: "bash".to_string(),
            tool_args: tool_args.clone(),
            result: ToolResult {
                tool_call_id,
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text { text: format!("result{}", i) }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        });
    }

    let tool_count = state.messages.iter().filter(|m| matches!(m, MessageItem::ToolCall { .. })).count();
    assert_eq!(tool_count, 3, "should have 3 tool calls");
}

/// Test: Tool with edit operation
#[test]
fn test_edit_tool() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "edit-1".to_string(),
            tool_name: "edit_file".to_string(),
            tool_args: r#"{"path": "src/main.rs", "old": "foo", "new": "bar"}"#.to_string(),
            turn: 1,
        },
    );

    let tool = state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            MessageItem::ToolCall { name, .. } if name == "edit_file" => Some(true),
            _ => None,
        });
    assert!(tool.is_some(), "edit tool should be present in messages");
}

/// Helper: execute a bash tool with multiple text parts
fn execute_bash_tool_multi_content(args: &str, parts: Vec<&str>) -> AppState {
    let mut state = make_test_state();
    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "list-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: args.to_string(),
            turn: 1,
        },
    );
    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "list-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: args.to_string(),
            result: ToolResult {
                tool_call_id: "list-1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: parts.into_iter().map(|t| ContentPart::Text { text: t.to_string() }).collect(),
                is_error: false,
            },
            duration_ms: 75,
            turn: 1,
        },
    );
    state
}

/// Test: Tool result preserves JSON content
#[test]
fn test_tool_result_json_content() {
    let state = execute_bash_tool_multi_content(
        r#"{"command": "ls -la"}"#,
        vec!["file1.txt", "file2.rs"],
    );

    let result_text = state.messages.iter().rev().find_map(|m| match m {
        MessageItem::ToolCall { name, result, .. } if name == "bash" => result.clone(),
        _ => None,
    });

    assert!(result_text.is_some(), "tool result should be present");
    let result = result_text.unwrap();
    assert!(result.contains("file1.txt"));
    assert!(result.contains("file2.rs"));
}

/// Test: Tool execution sets Working status
#[test]
fn test_tool_execution_sets_working_status() {
    let mut state = make_test_state();
    state.is_thinking = true;

    handle_agent_event(
        &mut state,
        AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "echo test"}"#.to_string(),
            turn: 1,
        },
    );

    // After tool start, status should be Running
    assert_eq!(
        state.status_header.as_deref(),
        Some("Running"),
        "status should be Running during tool execution"
    );
}