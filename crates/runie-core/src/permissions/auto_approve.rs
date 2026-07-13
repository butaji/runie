//! Auto-approve policy for `PermissionMode::Auto` (`/auto` mode).
//!
//! Allows read, edit and shell tools without confirmation, but still asks
//! when the tool targets a sensitive path. Unknown tools do not match and
//! fall through to the rest of the policy chain.

use async_trait::async_trait;

use super::{
    is_read_only_tool, is_sensitive_path, PermissionContext, PermissionPolicy, PermissionResult,
};

/// Auto-approve read, edit and shell tools; ask for sensitive paths.
#[derive(Debug, Default, Clone, Copy)]
pub struct AutoApprove;

impl AutoApprove {
    pub fn new() -> Self {
        Self
    }

    fn is_edit_tool(tool: &str) -> bool {
        matches!(tool, "write_file" | "edit_file")
    }

    fn is_shell_tool(tool: &str) -> bool {
        tool == "bash"
    }
}

#[async_trait]
impl PermissionPolicy for AutoApprove {
    fn name(&self) -> &str {
        "auto_approve"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        is_read_only_tool(ctx.tool)
            || Self::is_edit_tool(ctx.tool)
            || Self::is_shell_tool(ctx.tool)
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
