//! Process-wide registry for in-flight permission approvals.
//!
//! The agent turn blocks on a oneshot receiver while the TUI shows a modal
//! dialog. When the user chooses Allow/Deny/Always-allow, the UI resolves the
//! request and the receiver completes.

use std::collections::HashMap;
use parking_lot::Mutex;
use tokio::sync::oneshot;

use super::PermissionAction;

/// Holds pending oneshot senders keyed by request id.
#[derive(Debug, Default)]
pub struct ApprovalRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<PermissionAction>>>,
}

impl ApprovalRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new approval request and return the receiver that will
    /// complete when [`resolve`](Self::resolve) is called.
    pub fn register(&self, request_id: &str) -> oneshot::Receiver<PermissionAction> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(request_id.to_owned(), tx);
        rx
    }

    /// Resolve a pending approval request with the user's chosen action.
    /// Returns `true` if the request existed and was resolved.
    pub fn resolve(&self, request_id: &str, action: PermissionAction) -> bool {
        let mut pending = self.pending.lock();
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(action);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_sends_action_to_receiver() {
        let registry = ApprovalRegistry::new();
        let rx = registry.register("req-1");

        assert!(registry.resolve("req-1", PermissionAction::Allow));

        assert_eq!(rx.blocking_recv(), Ok(PermissionAction::Allow));
    }

    #[test]
    fn resolve_unknown_request_returns_false() {
        let registry = ApprovalRegistry::new();

        assert!(!registry.resolve("missing", PermissionAction::Deny));
    }

    #[test]
    fn multiple_requests_are_independent() {
        let registry = ApprovalRegistry::new();
        let rx_a = registry.register("a");
        let rx_b = registry.register("b");

        registry.resolve("a", PermissionAction::Allow);
        registry.resolve("b", PermissionAction::Deny);

        assert_eq!(rx_a.blocking_recv(), Ok(PermissionAction::Allow));
        assert_eq!(rx_b.blocking_recv(), Ok(PermissionAction::Deny));
    }
}
