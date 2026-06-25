//! Agent turn tests that exercise the TUI-style permission gate.

use crate::tests::ensure_mock_provider;
use crate::{run_agent_turn, AgentCommand};
use runie_core::permissions::{
    ApprovalSink, AutoAllowSink, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove,
    PermissionAction, PermissionManager,
};
use runie_core::Event;
use runie_testing::mock_provider;
use std::sync::{Arc, Mutex};

/// A mock approval sink that always denies (for testing without a real PermissionActor).
pub struct MockApprovalSink;

#[async_trait::async_trait]
impl ApprovalSink for MockApprovalSink {
    async fn ask(&self, _tool: &str, _input: &serde_json::Value) -> PermissionAction {
        PermissionAction::Deny
    }
}

/// Simulate the permission gate used by the TUI and verify read-only tools
/// are auto-approved and rendered as ToolStart/ToolEnd events.
#[tokio::test]
async fn test_agent_loop_with_tui_gate_allows_read_only_tool() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
    let cmd = AgentCommand {
        content: "list files".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    let emit = Arc::new(Mutex::new(move |evt: runie_core::event::Event| {
        events_clone.lock().unwrap().push(evt)
    }));
    let permissions = PermissionManager::default().with_policies(vec![
        Box::new(DefaultToolApprove::new()),
        Box::new(GitTrackedWriteApprove::new()),
        Box::new(FileAccessAsk::new()),
    ]);
    // Use AutoAllowSink so read-only tools are auto-approved without needing a real PermissionActor.
    let gate = crate::PermissionGate::new(
        permissions,
        Arc::new(AutoAllowSink),
    );

    run_agent_turn(&provider, &cmd, emit, 5, gate)
        .await
        .unwrap();

    let events = events.lock().unwrap();
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
