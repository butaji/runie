//! Process-wide registry for in-flight permission approvals.
//!
//! The agent turn blocks on a oneshot receiver while the TUI shows a modal
//! dialog. When the user chooses Allow/Deny/Always-allow, the UI resolves the
//! request and the receiver completes.
//!
//! Uses `parking_lot::Mutex` for synchronous access (suitable for use in actors).

use parking_lot::Mutex;
use std::collections::HashMap;
use tokio::sync::oneshot;

use super::PermissionAction;

/// Holds pending reply channels keyed by request id.
#[derive(Debug, Default)]
pub struct ApprovalRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<PermissionAction>>>,
}

impl ApprovalRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new approval request with the reply channel (sync).
    /// The reply channel will be used by [`resolve`](Self::resolve) to deliver the user's choice.
    pub fn register(&self, request_id: &str, reply: oneshot::Sender<PermissionAction>) {
        self.pending.lock().insert(request_id.to_owned(), reply);
    }

    /// Resolve a pending approval request with the user's chosen action (sync).
    /// Returns `true` if the request existed and was resolved.
    pub fn resolve(&self, request_id: &str, action: PermissionAction) -> bool {
        let mut pending = self.pending.lock();
        if let Some(reply) = pending.remove(request_id) {
            let _ = reply.send(action);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    #[test]
    fn resolve_sends_action_to_receiver() {
        let registry = ApprovalRegistry::new();
        let (tx, rx) = oneshot::channel();

        registry.register("req-1", tx);
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
        let (tx_a, rx_a) = oneshot::channel();
        let (tx_b, rx_b) = oneshot::channel();

        registry.register("a", tx_a);
        registry.register("b", tx_b);

        registry.resolve("a", PermissionAction::Allow);
        registry.resolve("b", PermissionAction::Deny);

        assert_eq!(rx_a.blocking_recv(), Ok(PermissionAction::Allow));
        assert_eq!(rx_b.blocking_recv(), Ok(PermissionAction::Deny));
    }
}
