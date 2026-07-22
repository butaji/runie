//! Approval sink that emits `PermissionRequest` events for TUI UX but always allows.
//!
//! The policy engine has been removed. This sink is kept so the TUI can still
//! show a permission dialog for UX, but tools always execute immediately.

use async_trait::async_trait;
use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::permissions::{ApprovalSink, PermissionAction};
use serde_json::Value;

/// Always-allow approval sink that emits events for TUI UX.
pub struct EmitApprovalSink {
    permission_handle: RactorPermissionHandle,
}

impl EmitApprovalSink {
    /// Create a new sink backed by the given permission actor handle.
    pub fn new(permission_handle: RactorPermissionHandle) -> Self {
        Self { permission_handle }
    }

    /// Cancel pending permission requests.
    pub fn cancel_pending(&self) {
        // No-op: there are no pending requests since ask() returns immediately.
    }
}

#[async_trait]
impl ApprovalSink for EmitApprovalSink {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction {
        // Emit the permission request event so the TUI can show a dialog.
        let request_id = uuid();
        let _rx = self
            .permission_handle
            .ask_permission(request_id, tool.to_owned(), input.clone())
            .await;
        // Always allow: policy engine removed, tools execute immediately.
        PermissionAction::Allow
    }
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("perm-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
