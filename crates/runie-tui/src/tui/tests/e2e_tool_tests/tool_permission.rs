use super::*;
use crate::tui::state::Msg;
use runie_agent::{AgentEvent, PermissionDecision, ContentPart};
use crate::components::MessageItem;

#[test]
fn test_e2e_tool_permission_allow() {
    let (mut state, mut palette) = create_test_app();
    let (tool_call_id, tool_name, tool_args) = ("call_1", "bash", r#"{"command": "ls"}"#);

    send_permission_request(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "Execute bash command",
    );

    assert_eq!(state.mode, TuiMode::Permission);
    assert_eq!(state.permission_modal.tool, Some(tool_name.to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::Allow { .. } }]));
    assert_eq!(state.mode, TuiMode::Chat);

    send_tool_execution_start(&mut state, &mut palette, tool_call_id, tool_name, tool_args);
    send_tool_execution_end(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "ls output", false, 50,
    );

    assert!(verify_last_tool_result_contains(&state, "ls output"));
}

#[test]
fn test_e2e_tool_permission_deny() {
    let (mut state, mut palette) = create_test_app();

    send_permission_request(
        &mut state, &mut palette,
        "call_deny", "bash", r#"{"command": "rm -rf /"}"#,
        "Execute dangerous command",
    );

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    assert_eq!(cmds.len(), 2);
    assert!(matches!(&cmds[0], Cmd::SendPermission { decision: PermissionDecision::Deny { .. } }));
    assert!(matches!(&cmds[1], Cmd::Rollback { tool_call_id } if tool_call_id == "call_deny"));

    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_tool_permission_allow_always() {
    let (mut state, mut palette) = create_test_app();
    let (tool_call_id, tool_name, tool_args) = ("call_first", "bash", r#"{"command": "ls"}"#);

    send_permission_request(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "Execute bash command",
    );

    assert_eq!(state.mode, TuiMode::Permission);

    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);
    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::AllowAlways { .. } }]));
    assert_eq!(state.mode, TuiMode::Chat);

    send_tool_execution_start(&mut state, &mut palette, tool_call_id, tool_name, tool_args);
    send_tool_execution_end(
        &mut state, &mut palette,
        tool_call_id, tool_name, tool_args,
        "result1", false, 50,
    );

    assert!(verify_last_tool_result_contains(&state, "result1"));
}

#[test]
fn test_e2e_permission_queue_multiple() {
    let (mut state, mut palette) = create_test_app();

    send_permission_request(
        &mut state, &mut palette,
        "call_1", "bash", r#"{"command": "ls"}"#,
        "Execute bash",
    );
    send_permission_request(
        &mut state, &mut palette,
        "call_2", "read_file", r#"{"path": "file.txt"}"#,
        "Read file",
    );

    assert_eq!(state.permission_modal.pending_queue.len(), 1);
    assert_eq!(state.permission_modal.pending_queue[0].tool_call_id, "call_2");

    assert_eq!(state.permission_modal.tool, Some("bash".to_string()));
    assert_eq!(state.permission_modal.tool_call_id, Some("call_1".to_string()));

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert!(matches!(&cmds[..], [Cmd::SendPermission { decision: PermissionDecision::Allow { .. } }]));

    assert_eq!(state.permission_modal.tool, Some("read_file".to_string()));
    assert_eq!(state.permission_modal.tool_call_id, Some("call_2".to_string()));
    assert!(state.permission_modal.pending_queue.is_empty());
}
