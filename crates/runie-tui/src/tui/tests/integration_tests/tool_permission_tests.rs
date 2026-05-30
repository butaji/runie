use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart, ToolResult};

/// Helper: execute a tool and get result
fn execute_tool(harness: AgentTestHarness, id: &str, name: &str, args: &str, result_text: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::ToolExecutionStart {
            tool_call_id: id.to_string(),
            tool_name: name.to_string(),
            tool_args: args.to_string(),
            turn: 1,
        })
        .handle_agent_event(AgentEvent::ToolExecutionEnd {
            tool_call_id: id.to_string(),
            tool_name: name.to_string(),
            tool_args: args.to_string(),
            result: ToolResult {
                tool_call_id: id.to_string(),
                tool_name: name.to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text { text: result_text.to_string() }],
                is_error: false,
            },
            duration_ms: 50,
            turn: 1,
        })
}

#[test]
fn test_agent_with_tool_use() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("List files");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });
    harness = execute_tool(harness, "t1", "bash", "ls", "file1.txt\nfile2.rs");

    harness.handle_agent_event(AgentEvent::MessageUpdate {
        message: super::helpers::agent_message("assistant", "Here are the files"),
        turn: 1,
        delta: "Here are the files".to_string(),
    });
    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: super::helpers::agent_message("assistant", "Here are the files"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 1,
        token_usage: super::helpers::default_token_usage(),
    });

    assert!(harness.state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })), "should have a tool call");
    assert!(harness.state.messages.iter().any(|m| matches!(m, MessageItem::Separator { .. })), "should have a separator");
}

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

#[test]
fn test_multiple_tools_in_one_turn() {
    let mut harness = AgentTestHarness::new();
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(10));

    harness.submit_user_message("List files and check git status");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: super::helpers::agent_message("assistant", ""),
        turn: 1,
    });

    harness = execute_tool(harness, "t1", "bash", "ls", "file1.txt\nfile2.rs");
    harness = execute_tool(harness, "t2", "bash", "git status", "On branch main");

    harness.handle_agent_event(AgentEvent::MessageEnd {
        message: super::helpers::agent_message("assistant", "I found 2 files and you're on main branch"),
        turn: 1,
    });
    harness.handle_agent_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 2,
        token_usage: super::helpers::default_token_usage(),
    });

    let tool_calls: Vec<_> = harness.state.messages.iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .collect();
    assert_eq!(tool_calls.len(), 2, "should have exactly 2 tool calls");
}
