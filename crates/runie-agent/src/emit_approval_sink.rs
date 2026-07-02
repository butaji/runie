//! Approval sink that emits a `PermissionRequest` event via `PermissionActor` and awaits a response.
//!
//! The sink sends `AskPermission` to the `PermissionActor` which:
//! 1. Registers the request with its internal `ApprovalRegistry`
//! 2. Emits `Event::PermissionRequest` to the bus
//! 3. Returns a oneshot receiver that completes when the user resolves the request
//!
//! The TUI resolves the request by sending `PermissionMsg::ResolvePermission` to the actor.
//!
//! ## Cancellation
//!
//! When the `CancellationToken` is cancelled (e.g. via `AbortTurn`), the pending
//! approval request is cancelled immediately and returns `PermissionAction::Deny`.

use async_trait::async_trait;
use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::permissions::{ApprovalSink, PermissionAction};
use serde_json::Value;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::constants::DEFAULT_PERMISSION_TIMEOUT_SECS;

/// Approval sink that delegates to `PermissionActor`.
pub struct EmitApprovalSink {
    permission_handle: RactorPermissionHandle,
    /// Timeout for permission requests in seconds. Defaults to 60 seconds.
    timeout_secs: u64,
    /// Abort signal: cancelling this token unblocks all pending `ask()` calls.
    cancel_token: CancellationToken,
}

impl EmitApprovalSink {
    /// Create a new sink backed by the given permission actor handle.
    pub fn new(permission_handle: RactorPermissionHandle) -> Self {
        Self::with_cancel(permission_handle, DEFAULT_PERMISSION_TIMEOUT_SECS, CancellationToken::new())
    }

    /// Create a new sink with a custom timeout.
    pub fn with_timeout(permission_handle: RactorPermissionHandle, timeout_secs: u64) -> Self {
        Self::with_cancel(permission_handle, timeout_secs, CancellationToken::new())
    }

    /// Create a new sink with a shared cancellation token.
    /// Cancelling the token aborts all pending approval requests.
    pub fn with_cancel(
        permission_handle: RactorPermissionHandle,
        timeout_secs: u64,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            permission_handle,
            timeout_secs,
            cancel_token,
        }
    }

    /// Cancel pending permission requests by cancelling the abort token.
    /// This unblocks any `ask()` call that is currently racing on the token.
    pub fn cancel_pending(&self) {
        self.cancel_token.cancel();
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

        let cancel = self.cancel_token.clone();
        let handle = self.permission_handle.clone();
        let req_id = request_id.clone();

        let result = tokio::select! {
            biased;
            // AbortTurn was fired — cancelled.
            _ = cancel.cancelled() => {
                let _ = handle.try_cancel_permission(req_id);
                None
            }
            result = rx => Some(result.ok()),
            _ = tokio::time::sleep(Duration::from_secs(self.timeout_secs)) => {
                // Timed out — deny.
                let _ = handle.try_cancel_permission(request_id.clone());
                None
            }
        };

        result.flatten().unwrap_or(PermissionAction::Deny)
    }
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("perm-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
