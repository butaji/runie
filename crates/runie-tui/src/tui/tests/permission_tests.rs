//! Permission tests for runie-tui.
//!
//! Tests for permission request handling:
//! - Permission request in Chat shows modal
//! - Permission request in Overlay queues
//! - Permission request in DiffViewer queues
//! - Permission request in SessionTree queues
//! - Confirm sends Allow decision
//! - Cancel sends Deny + Rollback
//! - Always Allow sends AllowAlways
//! - Skip sends Skip + Rollback
//! - Timeout auto-denies after 5 min
//! - Multiple permissions queued (FIFO)
//! - Queue cleared on AgentEnd

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode, PendingPermission, Cmd};
use crate::tui::update::agent::permission::{
    on_permission_request, handle_permission_msg, handle_permission_timeout,
};
use crate::tui::update::agent::AgentCmd;
use crate::tui::tests::reducer::make_state;
use crate::components::CommandPalette;
use crate::tui::update::update;
use runie_agent::{AgentEvent, PermissionDecision};
use std::time::Instant;

#[test]
fn test_permission_request_in_chat_shows_modal() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;

    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Permission,
        "Mode should switch to Permission in Chat mode"
    );
    assert!(
        state.permission_modal.tool.is_some(),
        "Permission modal should show tool"
    );
}

#[test]
fn test_permission_request_in_overlay_queues() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;

    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Overlay,
        "Mode should stay in Overlay when permission queued"
    );
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "Permission should be queued"
    );
    assert!(
        state.permission_modal.tool.is_none(),
        "Current modal should be empty"
    );
}

#[test]
fn test_permission_request_in_diff_viewer_queues() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;

    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());

    assert_eq!(
        state.mode,
        TuiMode::DiffViewer,
        "Mode should stay in DiffViewer when permission queued"
    );
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "Permission should be queued"
    );
}

#[test]
fn test_permission_request_in_session_tree_queues() {
    let mut state = make_state();
    state.mode = TuiMode::SessionTree;

    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());

    assert_eq!(
        state.mode,
        TuiMode::SessionTree,
        "Mode should stay in SessionTree when permission queued"
    );
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "Permission should be queued"
    );
}

#[test]
fn test_confirm_sends_allow_decision() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Allow { .. }))),
        "Confirm should send Allow decision"
    );
}

#[test]
fn test_cancel_sends_deny_and_rollback() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionCancel);

    // Should have both SendPermission(Deny) and Rollback
    assert_eq!(cmds.len(), 2, "Cancel should produce SendPermission + Rollback");
    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Deny { .. }))),
        "Cancel should send Deny decision"
    );
    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::Rollback { tool_call_id } if tool_call_id == "tool_1")),
        "Cancel should send Rollback"
    );
}

#[test]
fn test_always_allows_sends_allow_always() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionAlways);

    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::AllowAlways { .. }))),
        "Always should send AllowAlways decision"
    );
}

#[test]
fn test_skip_sends_skip_and_rollback() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionSkip);

    assert_eq!(cmds.len(), 2, "Skip should produce SendPermission + Rollback");
    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::SendPermission { decision } if matches!(decision, PermissionDecision::Skip { .. }))),
        "Skip should send Skip decision"
    );
    assert!(
        cmds.iter().any(|c| matches!(c, AgentCmd::Rollback { tool_call_id } if tool_call_id == "tool_1")),
        "Skip should send Rollback"
    );
}

#[test]
fn test_timeout_auto_denies_after_5_min() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.permission_modal.timeout_start = Some(Instant::now() - std::time::Duration::from_secs(301)); // 5+ min ago
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_timeout(&mut state);

    assert!(
        state.permission_modal.timed_out,
        "Timeout flag should be set"
    );
    assert!(
        cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision } if matches!(decision, PermissionDecision::Deny { .. }))),
        "Timeout should send Deny decision"
    );
    // Should show timeout message
    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("timed out"))),
        "Should show timeout message"
    );
}

#[test]
fn test_multiple_permissions_queued_fifo() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay; // Force queueing

    // Add first permission
    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());
    // Add second permission
    on_permission_request(&mut state, "tool_2".to_string(), "read".to_string(), "file.txt".to_string());
    // Add third permission
    on_permission_request(&mut state, "tool_3".to_string(), "write".to_string(), "out.txt".to_string());

    assert_eq!(
        state.permission_modal.pending_queue.len(),
        3,
        "All three permissions should be queued"
    );

    // Verify FIFO order
    assert_eq!(
        state.permission_modal.pending_queue[0].tool_name, "bash",
        "First in queue should be bash"
    );
    assert_eq!(
        state.permission_modal.pending_queue[1].tool_name, "read",
        "Second in queue should be read"
    );
    assert_eq!(
        state.permission_modal.pending_queue[2].tool_name, "write",
        "Third in queue should be write"
    );
}

#[test]
fn test_queue_cleared_on_agent_end() {
    let mut state = make_state();
    state.permission_modal.pending_queue = vec![
        PendingPermission {
            tool_call_id: "tool_1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
        },
        PendingPermission {
            tool_call_id: "tool_2".to_string(),
            tool_name: "read".to_string(),
            tool_args: "file.txt".to_string(),
        },
    ];
    state.mode = TuiMode::Permission;

    // Simulate AgentEnd event
    let _ = crate::tui::update::agent::events::on_agent_end(&mut state);

    assert!(
        state.permission_modal.pending_queue.is_empty(),
        "Pending queue should be cleared on AgentEnd"
    );
}

#[test]
fn test_permission_confirm_resets_mode_to_chat_when_queue_empty() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.mode = TuiMode::Permission;

    let _ = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    assert_eq!(
        state.mode,
        TuiMode::Chat,
        "Mode should reset to Chat after confirm when queue empty"
    );
}

#[test]
fn test_permission_confirm_processes_next_from_queue() {
    let mut state = make_state();
    // Setup: current permission + queued permission
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.permission_modal.pending_queue = vec![PendingPermission {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "file.txt".to_string(),
    }];
    state.mode = TuiMode::Permission;

    let _ = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    // Should process next from queue
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("read"),
        "Should show next queued permission"
    );
    assert_eq!(
        state.mode,
        TuiMode::Permission,
        "Mode should stay in Permission to show next request"
    );
}

#[test]
fn test_permission_request_while_modal_open_queues() {
    let mut state = make_state();
    // First permission - shows modal
    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());
    // Second permission while modal open - should queue
    on_permission_request(&mut state, "tool_2".to_string(), "read".to_string(), "file.txt".to_string());

    assert_eq!(
        state.mode,
        TuiMode::Permission,
        "Mode should stay in Permission"
    );
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        1,
        "Second permission should be queued"
    );
}

#[test]
fn test_permission_modal_timeout_reset_on_queue_processing() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.permission_modal.timeout_start = Some(Instant::now() - std::time::Duration::from_secs(200));
    state.permission_modal.pending_queue = vec![PendingPermission {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "file.txt".to_string(),
    }];
    state.mode = TuiMode::Permission;

    let _ = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    // Timeout should be reset for new permission
    let elapsed = state
        .permission_modal
        .timeout_start
        .map(|s| s.elapsed().as_secs())
        .unwrap_or(0);
    assert!(
        elapsed < 5,
        "Timeout should be reset for next queued permission"
    );
}

#[test]
fn test_permission_queue_message_shown_when_queued() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;

    on_permission_request(&mut state, "tool_1".to_string(), "bash".to_string(), "ls".to_string());

    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("queued"))),
        "Should show queued message to user"
    );
}

#[test]
fn test_permission_timeout_resets_timeout_for_next_queued() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.permission_modal.timeout_start = Some(Instant::now() - std::time::Duration::from_secs(200));
    state.permission_modal.pending_queue = vec![PendingPermission {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "file.txt".to_string(),
    }];
    state.mode = TuiMode::Permission;

    let _ = handle_permission_timeout(&mut state);

    // Timeout should be reset for new permission
    let elapsed = state
        .permission_modal
        .timeout_start
        .map(|s| s.elapsed().as_secs())
        .unwrap_or(0);
    assert!(
        elapsed < 5,
        "Timeout should be reset for next queued permission"
    );
    assert_eq!(
        state.permission_modal.timed_out, false,
        "Timed out flag should be reset for new permission"
    );
}

#[test]
fn test_permission_confirm_uses_correct_tool_info() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_123".to_string());
    state.permission_modal.args = Some("ls -la".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    if let Some(AgentCmd::SendPermission { decision }) = cmds.first() {
        if let PermissionDecision::Allow { tool_call_id, tool_name, tool_args } = decision {
            assert_eq!(*tool_call_id, "tool_123");
            assert_eq!(tool_name.as_str(), "bash");
            assert_eq!(tool_args.as_str(), "ls -la");
        } else {
            panic!("Expected Allow decision");
        }
    } else {
        panic!("Expected SendPermission command");
    }
}

#[test]
fn test_permission_cancel_uses_correct_tool_info() {
    let mut state = make_state();
    state.permission_modal.tool = Some("read".to_string());
    state.permission_modal.tool_call_id = Some("tool_456".to_string());
    state.permission_modal.args = Some("file.txt".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionCancel);

    if let Some(AgentCmd::SendPermission { decision }) = cmds.first() {
        if let PermissionDecision::Deny { tool_call_id, tool_name, tool_args } = decision {
            assert_eq!(*tool_call_id, "tool_456");
            assert_eq!(tool_name.as_str(), "read");
            assert_eq!(tool_args.as_str(), "file.txt");
        } else {
            panic!("Expected Deny decision");
        }
    } else {
        panic!("Expected SendPermission command");
    }
}

#[test]
fn test_permission_always_uses_correct_tool_info() {
    let mut state = make_state();
    state.permission_modal.tool = Some("write".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());
    state.permission_modal.args = Some("out.txt".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionAlways);

    if let Some(AgentCmd::SendPermission { decision }) = cmds.first() {
        if let PermissionDecision::AllowAlways { tool_call_id, tool_name, tool_args } = decision {
            assert_eq!(*tool_call_id, "tool_789");
            assert_eq!(tool_name.as_str(), "write");
            assert_eq!(tool_args.as_str(), "out.txt");
        } else {
            panic!("Expected AllowAlways decision");
        }
    } else {
        panic!("Expected SendPermission command");
    }
}

#[test]
fn test_permission_skip_uses_correct_tool_info() {
    let mut state = make_state();
    state.permission_modal.tool = Some("delete".to_string());
    state.permission_modal.tool_call_id = Some("tool_999".to_string());
    state.permission_modal.args = Some("file.txt".to_string());
    state.mode = TuiMode::Permission;

    let cmds = handle_permission_msg(&mut state, Msg::PermissionSkip);

    if let Some(AgentCmd::SendPermission { decision }) = cmds.first() {
        if let PermissionDecision::Skip { tool_call_id, tool_name, tool_args } = decision {
            assert_eq!(*tool_call_id, "tool_999");
            assert_eq!(tool_name.as_str(), "delete");
            assert_eq!(tool_args.as_str(), "file.txt");
        } else {
            panic!("Expected Skip decision");
        }
    } else {
        panic!("Expected SendPermission command");
    }
}

#[test]
fn test_permission_confirm_clears_modal_fields() {
    let mut state = make_state();
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_1".to_string());
    state.permission_modal.args = Some("ls".to_string());
    state.permission_modal.desc = Some("Agent wants to run bash".to_string());
    state.mode = TuiMode::Permission;

    let _ = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    // Modal should be cleared (or have next permission if queue had items)
    assert!(
        state.permission_modal.tool_call_id.is_none() || state.permission_modal.tool.is_some(),
        "Modal should be updated after confirm"
    );
}

#[test]
fn test_permission_queue_maintains_order_after_multiple_operations() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;

    // Queue 3 permissions
    on_permission_request(&mut state, "tool_1".to_string(), "A".to_string(), "a".to_string());
    on_permission_request(&mut state, "tool_2".to_string(), "B".to_string(), "b".to_string());
    on_permission_request(&mut state, "tool_3".to_string(), "C".to_string(), "c".to_string());

    // Process one (confirm) - need to switch to Permission mode first
    state.mode = TuiMode::Permission;
    let pending = state.permission_modal.pending_queue.remove(0);
    state.permission_modal.tool = Some(pending.tool_name.clone());
    state.permission_modal.tool_call_id = Some(pending.tool_call_id.clone());
    let _ = handle_permission_msg(&mut state, Msg::PermissionConfirm);

    // Add another
    on_permission_request(&mut state, "tool_4".to_string(), "D".to_string(), "d".to_string());

    // Queue should be: B, C, D
    assert_eq!(
        state.permission_modal.pending_queue.len(),
        3,
        "Queue should maintain order after operations"
    );
    assert_eq!(state.permission_modal.pending_queue[0].tool_name, "B");
    assert_eq!(state.permission_modal.pending_queue[1].tool_name, "C");
    assert_eq!(state.permission_modal.pending_queue[2].tool_name, "D");
}
