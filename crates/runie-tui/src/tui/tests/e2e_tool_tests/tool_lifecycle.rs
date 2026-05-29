use super::*;
use crate::tui::state::Msg;
use runie_agent::{AgentEvent, PermissionDecision, ContentPart};
use crate::components::MessageItem;

#[test]
fn test_e2e_bash_tool_full_flow() {
    let (mut state, mut palette) = create_test_app();
    let (tool_call_id, tool_name, tool_args) = ("call_1", "bash", r#"{"command": "ls"}"#);

    send_permission_request(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "Execute bash command",
    );

    assert_eq!(state.mode, TuiMode::Permission);
    assert_eq!(state.permission_modal.tool, Some(tool_name.to_string()));
    assert_eq!(state.permission_modal.tool_call_id, Some(tool_call_id.to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));

    send_tool_execution_start(&mut state, &mut palette, tool_call_id, tool_name, tool_args);
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { name, .. } if name == tool_call_id)));

    send_tool_execution_end(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "file1.txt\nfile2.rs", false, 50,
    );

    if let Some(MessageItem::ToolCall { result, is_error, .. }) = state.messages.last() {
        assert!(result.is_some(), "ToolCall should have result");
        assert!(!is_error, "ToolCall is_error should be false");
        assert!(result.as_ref().unwrap().contains("file1.txt"));
    } else {
        panic!("Expected ToolCall message");
    }
}

#[test]
fn test_e2e_read_file_tool_flow() {
    let (mut state, mut palette) = create_test_app();
    let (tool_call_id, tool_name, tool_args) = ("call_read_1", "read_file", r#"{"path": "src/main.rs"}"#);

    send_permission_request(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "Read file contents",
    );

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision: PermissionDecision::Allow { .. } })));

    send_tool_execution_start(&mut state, &mut palette, tool_call_id, tool_name, tool_args);

    send_tool_execution_end(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "fn main() {}", false, 10,
    );

    assert!(verify_last_tool_result_contains(&state, "fn main()"));
}

#[test]
fn test_e2e_tool_chain_multiple() {
    let (mut state, mut palette) = create_test_app();

    send_tool_execution_start(&mut state, &mut palette, "call_a", "bash", r#"{"command": "ls"}"#);
    send_tool_execution_end(
        &mut state, &mut palette,
        "call_a", "bash", r#"{"command": "ls"}"#,
        "output_a", false, 50,
    );

    send_tool_execution_start(&mut state, &mut palette, "call_b", "read_file", r#"{"path": "output_a"}"#);
    send_tool_execution_end(
        &mut state, &mut palette,
        "call_b", "read_file", r#"{"path": "output_a"}"#,
        "content_of_file", false, 30,
    );

    let tool_calls: Vec<_> = state
        .messages
        .iter()
        .filter_map(|m| {
            if let MessageItem::ToolCall { name, result, .. } = m {
                Some((name.clone(), result.clone()))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0].0, "call_a");
    assert!(tool_calls[0].1.as_ref().unwrap().contains("output_a"));
    assert_eq!(tool_calls[1].0, "call_b");
    assert!(tool_calls[1].1.as_ref().unwrap().contains("content_of_file"));
}
