//! Agent turn tests that exercise the TUI-style permission gate.

use crate::tests::ensure_mock_provider;
use crate::{agent_command_builder::agent_cmd, run_agent_turn};
use parking_lot::Mutex;
use runie_core::permissions::{
    AutoAllowSink, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};
use runie_core::Event;
use runie_testing::mock_provider;
use std::sync::Arc;

/// Simulate the permission gate used by the TUI and verify read-only tools
/// are auto-approved and rendered as ToolStart/ToolEnd events.
#[tokio::test]
async fn test_agent_loop_with_tui_gate_allows_read_only_tool() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("list files").build();

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let emit = Arc::new(Mutex::new(move |evt: runie_core::event::Event| {
        events_clone.lock().push(evt)
    }));
    let permissions = PermissionManager::default().with_policies(vec![
        Box::new(DefaultToolApprove::new()),
        Box::new(GitTrackedWriteApprove::new()),
        Box::new(FileAccessAsk::new()),
    ]);
    // Use AutoAllowSink so read-only tools are auto-approved without needing a real PermissionActor.
    let gate = crate::PermissionGate::new(permissions, Arc::new(AutoAllowSink));

    run_agent_turn(&provider, &cmd, emit, 5, gate)
        .await
        .unwrap();

    let events = events.lock();
    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, Event::ToolStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, Event::ToolEnd { .. }))
        .count();
    assert!(tool_starts >= 1, "expected at least one ToolStart");
    assert_eq!(tool_starts, tool_ends, "ToolStart/ToolEnd mismatch");
}
