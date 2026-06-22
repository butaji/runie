//! Permission gate combining a policy chain with an approval sink.

use std::sync::Arc;

use super::{ApprovalSink, PermissionAction, PermissionContext, PermissionManager, PermissionResult};

/// Combines a permission policy chain with an approval sink.
#[derive(Clone)]
pub struct PermissionGate {
    manager: Arc<PermissionManager>,
    sink: Arc<dyn ApprovalSink>,
}

impl PermissionGate {
    /// Create a permission gate from a manager and an approval sink.
    pub fn new(manager: PermissionManager, sink: Arc<dyn ApprovalSink>) -> Self {
        Self {
            manager: Arc::new(manager),
            sink,
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

    /// Get a reference to the approval sink (for testing).
    pub fn sink_ref(&self) -> &Arc<dyn ApprovalSink> {
        &self.sink
    }
}
