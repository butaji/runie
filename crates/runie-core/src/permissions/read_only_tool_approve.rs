//! Auto-approve read-only tools policy for `PermissionMode::Default`.
//!
//! Read-only tools are safe to auto-approve regardless of file path,
//! since they cannot modify system state.

use async_trait::async_trait;

use super::{is_sensitive_path, PermissionContext, PermissionPolicy, PermissionResult};

/// Auto-approve read-only tools; ask for sensitive paths.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReadOnlyToolApprove;

impl ReadOnlyToolApprove {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PermissionPolicy for ReadOnlyToolApprove {
    fn name(&self) -> &str {
        "read_only_tool_approve"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        super::is_read_only_tool(ctx.tool)
    }

    async fn evaluate(&self, ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        // High-risk targets still require confirmation.
        if let Some(path) = ctx.path {
            if is_sensitive_path(&path.to_string_lossy()) {
                return Some(PermissionResult::Ask);
            }
        }
        Some(PermissionResult::Allow)
    }
}
