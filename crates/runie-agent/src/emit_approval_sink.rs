//! Approval sink that emits a `PermissionRequest` event and denies by default.
//!
//! This sink lets the TUI show a permission dialog without blocking the agent
//! turn. A future iteration can wire the `PermissionResponse` event back into
/// the sink to await user approval.

use async_trait::async_trait;
use runie_core::event::Event;
use runie_core::permissions::{ApprovalSink, PermissionAction};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Approval sink that emits a permission request event.
pub struct EmitApprovalSink {
    emit: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>,
}

impl EmitApprovalSink {
    /// Create a new sink backed by the given emit callback.
    pub fn new(emit: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>) -> Self {
        Self { emit }
    }
}

#[async_trait]
impl ApprovalSink for EmitApprovalSink {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction {
        let request_id = uuid();
        let event = Event::PermissionRequest {
            request_id,
            tool: tool.to_string(),
            input: input.clone(),
        };
        if let Ok(mut emit) = self.emit.lock() {
            emit(event);
        }
        // Safe default: deny until a response channel is wired.
        PermissionAction::Deny
    }
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("perm-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
