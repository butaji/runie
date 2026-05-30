//! Comprehensive test suite - Section 5: Permission Timeout Tests (pi pattern).

use crate::components::MessageItem;
use crate::tui::state::{TuiMode, Cmd};
use crate::tui::update::system::check_permission_timeout;

use super::harness::AgentTestHarness;
use super::state_tests::make_message;

#[test]
fn test_permission_timeout_denies() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );
    harness.state.permission_modal.tool_call_id = Some("tool_123".to_string());

    let cmds = check_permission_timeout(&mut harness.state);

    assert!(!cmds.is_empty(), "Timeout should produce commands");
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { decision } if matches!(decision, runie_agent::PermissionDecision::Deny { .. }))));
}

#[test]
fn test_permission_timeout_clears_modal() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );
    harness.state.permission_modal.tool = Some("bash".to_string());
    harness.state.permission_modal.tool_call_id = Some("tool_123".to_string());

    check_permission_timeout(&mut harness.state);

    assert!(harness.state.permission_modal.tool.is_none());
}

#[test]
fn test_permission_timeout_queues_next() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );
    harness.state.permission_modal.tool_call_id = Some("tool_1".to_string());
    harness.state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "tool_2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "test.txt".to_string(),
    });

    check_permission_timeout(&mut harness.state);

    assert_eq!(harness.state.permission_modal.tool, Some("read_file".to_string()));
    assert!(harness.state.permission_modal.pending_queue.is_empty());
}

#[test]
fn test_no_timeout_when_not_permission_mode() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Chat;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );

    let cmds = check_permission_timeout(&mut harness.state);

    assert!(cmds.is_empty(), "Non-permission mode should not timeout");
}

#[test]
fn test_no_timeout_within_limit() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(100)
    );

    let cmds = check_permission_timeout(&mut harness.state);

    assert!(cmds.is_empty(), "Within timeout limit should not trigger");
    assert_eq!(harness.state.mode, TuiMode::Permission);
}

#[test]
fn test_timeout_adds_system_message() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );
    harness.state.permission_modal.tool_call_id = Some("tool_123".to_string());

    check_permission_timeout(&mut harness.state);

    let has_timeout_msg = harness.state.messages.iter().any(|m| {
        matches!(m, MessageItem::System { text } if text.contains("timed out"))
    });

    assert!(has_timeout_msg);
}

#[test]
fn test_already_timed_out_no_duplicate() {
    let mut harness = AgentTestHarness::new();
    harness.state.mode = TuiMode::Permission;
    harness.state.permission_modal.timeout_start = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(301)
    );
    harness.state.permission_modal.timed_out = true;

    let cmds = check_permission_timeout(&mut harness.state);

    assert!(cmds.is_empty(), "Already timed out should not trigger again");
}
