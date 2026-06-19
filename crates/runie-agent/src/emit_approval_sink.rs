//! Approval sink that emits a `PermissionRequest` event and awaits a response.
//!
//! The sink registers itself in the app state's [`ApprovalRegistry`] before
//! emitting the request. The TUI resolves the request when the user chooses an
//! action, completing the oneshot receiver and unblocking the agent turn.
use async_trait::async_trait;
use runie_core::event::Event;
use runie_core::permissions::{ApprovalRegistry, ApprovalSink, PermissionAction};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Approval sink that emits a permission request event.
pub struct EmitApprovalSink {
    emit: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>,
    registry: Arc<Mutex<ApprovalRegistry>>,
}

impl EmitApprovalSink {
    /// Create a new sink backed by the given emit callback and approval registry.
    pub fn new(
        emit: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>,
        registry: Arc<Mutex<ApprovalRegistry>>,
    ) -> Self {
        Self { emit, registry }
    }
}

#[async_trait]
impl ApprovalSink for EmitApprovalSink {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction {
        let request_id = uuid();
        let rx = {
            let registry = self.registry.lock().expect("approval registry poisoned");
            registry.register(&request_id)
        };
        let event = Event::PermissionRequest {
            request_id,
            tool: tool.to_string(),
            input: input.clone(),
        };
        if let Ok(mut emit) = self.emit.lock() {
            emit(event);
        }

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
