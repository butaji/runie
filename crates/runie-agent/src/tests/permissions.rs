//! Permission system tests.
//!
//! Policy engine removed — tools always bypass. These tests verify the stubs
//! and utility functions that remain.

use runie_core::permissions::is_read_only_tool;

#[test]
fn read_only_tools_recognized() {
    for tool in ["read_file", "list_dir", "grep", "find", "fetch_docs"] {
        assert!(is_read_only_tool(tool), "{tool} should be read-only");
    }
    assert!(!is_read_only_tool("write_file"));
    assert!(!is_read_only_tool("bash"));
}

/// Gate always evaluates to Allow (policy engine removed).
#[tokio::test]
async fn gate_always_allows() {
    let gate = crate::PermissionGate::new(std::sync::Arc::new(
        runie_core::permissions::AutoAllowSink,
    ));
    let ctx = runie_core::permissions::PermissionContext {
        tool: "bash",
        path: None,
        input: None,
        cwd: None,
        #[cfg(feature = "mcp")]
        annotations: None,
    };
    assert_eq!(gate.evaluate(&ctx).await, runie_core::permissions::PermissionAction::Allow);
}
