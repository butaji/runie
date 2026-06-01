//! Permission tests.
//!
//! Tests:
//! - Permission request switches to Permission mode
//! - Permission confirm/deny/always/skip
//! - Permission timeout (5 min)
//! - Permission queue (multiple pending, FIFO)
//! - Queue cleared on AgentEnd

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::TuiMode;
use crate::tui::update::agent::handle_agent_event;
use crate::tui::update::agent::permission::handle_permission;
use runie_agent::{AgentEvent, PermissionDecision};

/// Helper: Create AppState ready for permission testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state.agent_running = true;
    state
}

// ─── Permission request tests ────────────────────────────────────────────────

#[test]
fn test_permission_request_switches_to_permission_mode() {
    let mut state = make_test_state();
    assert_eq!(state.mode, TuiMode::Chat, "should start in Chat mode");

    handle_agent_event(
        &mut state,
        AgentEvent::PermissionRequest {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "rm -rf /"}"#.to_string(),
            tool_description: "Remove directory".to_string(),
            turn: 1,
            context_window_usage: 0.5,
        },
    );

    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission");
}

#[test]
fn test_permission_request_sets_modal_state() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::PermissionRequest {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls"}"#.to_string(),
            tool_description: "List files".to_string(),
            turn: 1,
            context_window_usage: 0.3,
        },
    );

    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("bash"),
        "tool should be set"
    );
    assert_eq!(
        state.permission_modal.tool_call_id.as_deref(),
        Some("call-1"),
        "tool_call_id should be set"
    );
    assert!(
        state.permission_modal.timeout_start.is_some(),
        "timeout_start should be set"
    );
    assert!(
        !state.permission_modal.timed_out,
        "timed_out should be false"
    );
}

#[test]
fn test_permission_request_adds_system_message() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::PermissionRequest {
            tool_call_id: "call-1".to_string(),
            tool_name: "read_file".to_string(),
            tool_args: r#"{"path": "/etc/passwd"}"#.to_string(),
            tool_description: "Read file".to_string(),
            turn: 1,
            context_window_usage: 0.2,
        },
    );

    // Permission request switches mode and sets up modal - no system message added
    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission");
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("read_file"),
        "tool should be set"
    );
}

// ─── Permission handling tests ───────────────────────────────────────────────

#[test]
fn test_permission_confirm_sends_allow_decision() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::Allow {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls"}"#.to_string(),
        },
    );

    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat");
    assert!(state.permission_modal.tool.is_none(), "modal should be cleared");
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Allow { .. }))),
        "should send Allow decision"
    );
}

#[test]
fn test_permission_deny_sends_deny_decision() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "rm -rf /"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::Deny {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "rm -rf /"}"#.to_string(),
        },
    );

    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat");
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Deny { .. }))),
        "should send Deny decision"
    );
}

#[test]
fn test_permission_always_sends_allow_always_decision() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::AllowAlways {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls"}"#.to_string(),
        },
    );

    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::AllowAlways { .. }))),
        "should send AllowAlways decision"
    );
}

#[test]
fn test_permission_skip_sends_skip_decision() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::Skip {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls"}"#.to_string(),
        },
    );

    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Skip { .. }))),
        "should send Skip decision"
    );
}

#[test]
fn test_permission_deny_triggers_rollback() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "bad"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::Deny {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "bad"}"#.to_string(),
        },
    );

    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::Rollback { tool_call_id } if tool_call_id == "call-1")),
        "Deny should trigger rollback"
    );
}

#[test]
fn test_permission_skip_triggers_rollback() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "bad"}"#.to_string());

    let cmds = handle_permission(
        &mut state,
        PermissionDecision::Skip {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "bad"}"#.to_string(),
        },
    );

    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::update::agent::AgentCmd::Rollback { .. })),
        "Skip should trigger rollback"
    );
}

// ─── Permission queue tests ───────────────────────────────────────────────────

#[test]
fn test_permission_queue_fifo() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    // Queue additional permissions
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "/etc/passwd"}"#.to_string(),
    });
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-3".to_string(),
        tool_name: "write_file".to_string(),
        tool_args: r#"{"path": "/tmp/out"}"#.to_string(),
    });

    // Confirm first permission
    let _cmds = handle_permission(
        &mut state,
        PermissionDecision::Allow {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: r#"{"command": "ls"}"#.to_string(),
        },
    );

    // Should auto-show next permission (call-2)
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("read_file"),
        "next queued permission should be shown"
    );
    assert_eq!(
        state.permission_modal.tool_call_id.as_deref(),
        Some("call-2"),
        "call-2 should be active"
    );
    assert_eq!(state.mode, TuiMode::Permission, "should still be in Permission mode");
}

#[test]
fn test_permission_queue_cleared_on_agent_end() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    // Add queued permissions
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-2".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{}".to_string(),
    });
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-3".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "{}".to_string(),
    });

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        },
    );

    assert!(
        state.permission_modal.pending_queue.is_empty(),
        "pending queue should be cleared"
    );
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat");
}

#[test]
fn test_permission_request_while_in_permission_queues() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());

    // Another permission request while one is active
    handle_agent_event(
        &mut state,
        AgentEvent::PermissionRequest {
            tool_call_id: "call-2".to_string(),
            tool_name: "read_file".to_string(),
            tool_args: "{}".to_string(),
            tool_description: "Read file".to_string(),
            turn: 1,
            context_window_usage: 0.3,
        },
    );

    // Should be queued, not replace current
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("bash"),
        "current permission should not be replaced"
    );
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "new request should be queued"
    );
}

// ─── Permission timeout tests ─────────────────────────────────────────────────

#[test]
fn test_permission_timeout_handling() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    // Use handle_permission_timeout
    let cmds = crate::tui::update::agent::handle_permission_timeout(&mut state);

    assert!(
        state.permission_modal.timed_out,
        "timed_out flag should be set"
    );
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat");
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("timed out")
        )),
        "should have timeout system message"
    );
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { .. })),
        "should send Deny for timeout"
    );
}

#[test]
fn test_permission_timeout_with_queued_requests() {
    let mut state = make_test_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool_call_id = Some("call-1".to_string());
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.args = Some(r#"{"command": "ls"}"#.to_string());

    // Queue another permission
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "{}".to_string(),
    });

    crate::tui::update::agent::handle_permission_timeout(&mut state);

    // Should auto-show next queued permission
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("read_file"),
        "next queued permission should be shown after timeout"
    );
    assert_eq!(
        state.permission_modal.tool_call_id.as_deref(),
        Some("call-2"),
        "call-2 should be active"
    );
    assert_eq!(state.mode, TuiMode::Permission, "should still be in Permission mode");
}

// ─── Permission in blocking mode queues ──────────────────────────────────────

#[test]
fn test_permission_request_in_blocking_mode_queues() {
    let mut state = make_test_state();
    state.mode = TuiMode::Overlay; // Blocking mode

    handle_agent_event(
        &mut state,
        AgentEvent::PermissionRequest {
            tool_call_id: "call-1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
            tool_description: "Run bash".to_string(),
            turn: 1,
            context_window_usage: 0.3,
        },
    );

    // Should be queued, not switch mode
    assert_eq!(state.mode, TuiMode::Overlay, "mode should remain Overlay");
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "permission should be queued"
    );
    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("queued")
        )),
        "should notify user about queue"
    );
}

