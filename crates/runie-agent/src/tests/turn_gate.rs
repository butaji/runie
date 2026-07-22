//! Agent turn tests that exercise the TUI-style permission gate.
//!
//! Policy engine removed — gate always delegates to the sink (AutoAllowSink in tests).

use crate::tests::ensure_mock_provider;
use crate::{agent_command_builder::agent_cmd, run_agent_turn};
use runie_core::event::Event;
use runie_core::permissions::AutoAllowSink;
use runie_testing::event_helpers::count_events;
use runie_testing::{capture_events, mock_provider};

/// Verify read-only tools are executed and render as ToolStart/ToolEnd events.
#[tokio::test]
async fn test_agent_loop_with_tui_gate_allows_read_only_tool() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
    let cmd = agent_cmd("list files").build();

    let (events, emit) = capture_events();
    let gate = crate::PermissionGate::new(std::sync::Arc::new(AutoAllowSink));

    run_agent_turn(&provider, &cmd, emit, 5, gate).await.unwrap();

    let tool_starts = count_events(&events, |e| matches!(e, Event::ToolStart { .. }));
    let tool_ends = count_events(&events, |e| matches!(e, Event::ToolEnd { .. }));
    assert!(tool_starts >= 1, "expected at least one ToolStart");
    assert_eq!(tool_starts, tool_ends, "ToolStart/ToolEnd mismatch");
}
