//! Permission gate combining a policy chain with an approval sink.

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::{
    ApprovalSink, PermissionAction, PermissionContext, PermissionManager, PermissionResult,
};

/// Combines a permission policy chain with an approval sink.
#[derive(Clone)]
pub struct PermissionGate {
    manager: Arc<PermissionManager>,
    sink: Arc<dyn ApprovalSink>,
    /// Abort signal: cancelling this token cancels pending permission requests.
    cancel_token: CancellationToken,
}

impl PermissionGate {
    /// Create a permission gate from a manager and an approval sink (no abort signal).
    pub fn new(manager: PermissionManager, sink: Arc<dyn ApprovalSink>) -> Self {
        Self::with_cancel(manager, sink, CancellationToken::new())
    }

    /// Create a permission gate with a shared cancellation token.
    pub fn with_cancel(
        manager: PermissionManager,
        sink: Arc<dyn ApprovalSink>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            manager: Arc::new(manager),
            sink,
            cancel_token,
        }
    }

    /// Evaluate the context against the policy chain and sink.
    pub async fn evaluate(&self, ctx: &PermissionContext<'_>) -> PermissionAction {
        match self.manager.evaluate(ctx).await {
            PermissionResult::Allow => PermissionAction::Allow,
            PermissionResult::Deny => PermissionAction::Deny,
            PermissionResult::Ask => {
                self.sink
                    .ask(ctx.tool, ctx.input.unwrap_or(&serde_json::Value::Null))
                    .await
            }
        }
    }

    /// Cancel any pending approval request.
    /// Call this when the turn is aborted (e.g. via AbortTurn / ForceQuit).
    pub fn cancel_pending(&self) {
        self.cancel_token.cancel();
    }
}
