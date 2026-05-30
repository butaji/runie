//! Comprehensive test suite - Section 6: Tool Execution Tests (crush + pi).

use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart, ToolResult};

use super::harness::AgentTestHarness;
use super::state_tests::make_message;

#[test]
fn test_tool_lifecycle() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run ls");

    // Tool starts
    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    // Assert tool in messages
    assert!(harness.state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolCall { name, .. } if name == "t1"
    )));

    // Tool ends
    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
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
        duration_ms: 100,
        turn: 1,
    });

    // Assert result updated
    let tool = harness
        .state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            MessageItem::ToolCall {
                name,
                result: Some(res),
                ..
            } if name == "t1" => Some(res.as_str()),
            _ => None,
        });
    assert!(tool.is_some());
    assert!(tool.unwrap().contains("file1.txt"));
}

#[test]
fn test_tool_with_error_result() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run failing command");

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "exit 1".to_string(),
        result: ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "Error: command failed".to_string(),
            }],
            is_error: true,
        },
        duration_ms: 50,
        turn: 1,
    });

    let error_tool = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall {
            name,
            is_error: true,
            ..
        } if name == "t1" => Some(true),
        _ => None,
    });
    assert!(error_tool.is_some());
}

#[test]
fn test_multiple_tools_in_sequence() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run commands");

    // Tool 1
    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo 1".to_string(),
        turn: 1,
    });
    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo 1".to_string(),
        result: ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "1".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 10,
        turn: 1,
    });

    // Tool 2
    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo 2".to_string(),
        turn: 1,
    });
    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo 2".to_string(),
        result: ToolResult {
            tool_call_id: "t2".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "2".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 10,
        turn: 1,
    });

    let tool_calls: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();

    assert_eq!(tool_calls.len(), 2);
}

#[test]
fn test_tool_pauses_thinking() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("List files");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    assert!(harness.state.is_thinking, "should be thinking after MessageStart");

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    assert!(!harness.state.is_thinking, "should NOT be thinking after ToolStart");
    assert!(
        harness.state.thinking_duration.is_some(),
        "thinking_duration should be recorded"
    );
}

#[test]
fn test_tool_updates_last_tool_call() {
    let mut harness = AgentTestHarness::new();

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo hi".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "echo hi".to_string(),
        result: ToolResult {
            tool_call_id: "t2".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text {
                text: "hi".to_string(),
            }],
            is_error: false,
        },
        duration_ms: 10,
        turn: 1,
    });

    // Result should be on t2, not t1
    let t2 = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall {
            name: t2_name,
            result: Some(_),
            ..
        } if t2_name == "t2" => Some(true),
        _ => None,
    });
    assert!(t2.is_some());
}

#[test]
fn test_tool_stores_args() {
    let mut harness = AgentTestHarness::new();

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "src/main.rs".to_string(),
        turn: 1,
    });

    let tool_args = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall {
            name: t1_name,
            args,
            ..
        } if t1_name == "t1" => Some(args.as_str()),
        _ => None,
    });

    assert_eq!(tool_args, Some("src/main.rs"));
}

#[test]
fn test_tool_empty_result() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Run command");

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "true".to_string(),
        turn: 1,
    });

    harness = harness.handle_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "true".to_string(),
        result: ToolResult {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![],
            is_error: false,
        },
        duration_ms: 10,
        turn: 1,
    });

    let result = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::ToolCall {
            name: t1_name,
            result: Some(res),
            ..
        } if t1_name == "t1" => Some(res.as_str()),
        _ => None,
    });

    assert_eq!(result, Some(""));
}
