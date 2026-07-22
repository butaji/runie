//! Simple permission gate — always bypasses to the approval sink.
//!
//! The policy chain has been removed. All tool calls are allowed immediately
//! via the sink. In TUI mode the sink returns `Ask` so the TUI can still
//! show a dialog (UX preserved), but headless always uses `AutoAllowSink`.

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::{ApprovalSink, PermissionAction, PermissionContext};

/// Permission gate — bypasses all checks by delegating directly to the sink.
#[derive(Clone)]
pub struct PermissionGate {
    sink: Arc<dyn ApprovalSink>,
    cancel_token: CancellationToken,
}

impl PermissionGate {
    /// Create a permission gate with the given approval sink.
    pub fn new(sink: Arc<dyn ApprovalSink>) -> Self {
        Self::with_cancel(sink, CancellationToken::new())
    }

    /// Create a permission gate with a shared cancellation token.
    pub fn with_cancel(sink: Arc<dyn ApprovalSink>, cancel_token: CancellationToken) -> Self {
        Self { sink, cancel_token }
    }

    /// Evaluate the context — always delegates to the sink (bypass all).
    pub async fn evaluate(&self, ctx: &PermissionContext<'_>) -> PermissionAction {
        self.sink
            .ask(ctx.tool, ctx.input.unwrap_or(&serde_json::Value::Null))
            .await
    }

    /// Cancel any pending approval request.
    pub fn cancel_pending(&self) {
        self.cancel_token.cancel();
    }

    /// Get a reference to the approval sink.
    pub fn sink_ref(&self) -> &Arc<dyn ApprovalSink> {
        &self.sink
    }

    /// Clone this gate for a subagent, inheriting the parent's sink.
    pub fn clone_for_subagent(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
            cancel_token: CancellationToken::new(),
        }
    }
}
