//! Auto-approve read-only / safe tools.

use async_trait::async_trait;

use super::{is_read_only_tool, PermissionContext, PermissionPolicy, PermissionResult};

/// Auto-approve safe/read-only tools.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultToolApprove;

impl DefaultToolApprove {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PermissionPolicy for DefaultToolApprove {
    fn name(&self) -> &str {
        "default_tool_approve"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        is_read_only_tool(ctx.tool)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Allow)
    }
}
