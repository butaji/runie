//! Agent turn tests that exercise the TUI-style permission gate.

use crate::{run_agent_turn, AgentCommand};
use runie_core::event::AgentEvent;
use runie_core::permissions::{
    ApprovalRegistry, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};
use crate::tests::ensure_mock_provider;
use runie_testing::mock_provider;
use std::sync::{Arc, Mutex};

/// Simulate the permission gate used by the TUI and verify read-only tools
/// are auto-approved and rendered as ToolStart/ToolEnd events.
#[tokio::test]
async fn test_agent_loop_with_tui_gate_allows_read_only_tool() {
    ensure_mock_provider();
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
    let approval_registry = Arc::new(Mutex::new(ApprovalRegistry::default()));
    let permissions = PermissionManager::default().with_policies(vec![
        Box::new(DefaultToolApprove::new()),
        Box::new(GitTrackedWriteApprove::new()),
        Box::new(FileAccessAsk::new()),
    ]);
    let gate = crate::PermissionGate::new(
        permissions,
        Arc::new(crate::emit_approval_sink::EmitApprovalSink::new(
            emit.clone(),
            approval_registry,
        )),
    );

    run_agent_turn(&provider, &cmd, emit, 5, gate).await.unwrap();

    let events = events.lock().unwrap();
    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolEnd { .. }))
        .count();
    assert!(tool_starts >= 1, "expected at least one ToolStart");
    assert_eq!(tool_starts, tool_ends, "ToolStart/ToolEnd mismatch");
}
