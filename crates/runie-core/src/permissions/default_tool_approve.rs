//! Auto-approve read-only / safe tools.
//!
//! Requires the `mcp` feature (uses `ToolAnnotations.read_only_hint`).

#![cfg(feature = "mcp")]
//!
//! Uses MCP `ToolAnnotations.read_only_hint` from the permission context to
//! determine whether a tool is safe for auto-approval.

use async_trait::async_trait;

use super::{PermissionContext, PermissionPolicy, PermissionResult};

/// Auto-approve tools where `ctx.annotations.read_only_hint == Some(true)`.
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
        // Read-only if annotations indicate it, or if no annotations known and tool
        // name suggests read-only.
        ctx.annotations
            .as_ref()
            .map(|a| a.read_only_hint == Some(true))
            .unwrap_or(false)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Allow)
    }
}
