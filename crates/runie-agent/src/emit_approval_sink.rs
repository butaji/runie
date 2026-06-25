//! Approval sink that emits a `PermissionRequest` event via `PermissionActor` and awaits a response.
//!
//! The sink sends `AskPermission` to the `PermissionActor` which:
//! 1. Registers the request with its internal `ApprovalRegistry`
//! 2. Emits `Event::PermissionRequest` to the bus
//! 3. Returns a oneshot receiver that completes when the user resolves the request
//!
//! The TUI resolves the request by sending `PermissionMsg::ResolvePermission` to the actor.

use async_trait::async_trait;
use runie_core::actors::PermissionActorHandle;
use runie_core::permissions::{ApprovalSink, PermissionAction};
use serde_json::Value;
use std::time::Duration;

/// Approval sink that delegates to `PermissionActor`.
pub struct EmitApprovalSink {
    permission_handle: PermissionActorHandle,
}

impl EmitApprovalSink {
    /// Create a new sink backed by the given permission actor handle.
    pub fn new(permission_handle: PermissionActorHandle) -> Self {
        Self { permission_handle }
    }
}

#[async_trait]
impl ApprovalSink for EmitApprovalSink {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction {
        let request_id = uuid();
        let rx = self
            .permission_handle
            .ask_permission(request_id.clone(), tool.to_owned(), input.clone())
            .await;

        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(action)) => action,
            _ => PermissionAction::Deny,
        }
    }
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("perm-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
