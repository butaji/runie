//! Permission decision coordination between the TUI and the agent loop.
//!
//! Replaces the previous `Arc<Mutex<Option<PermissionDecision>>>` + 100ms
//! polling pattern with a `tokio::sync::Notify`-driven wait. Decisions are
//! matched on `tool_call_id` so a stale decision for a different tool call
//! cannot accidentally resolve a fresh request.

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use crate::events::PermissionDecision;

/// Shared permission state. The TUI calls [`PermissionState::resolve`] when
/// the user makes a decision; the agent loop awaits [`PermissionState::wait`]
/// in `request_permission`.
pub struct PermissionState {
    inner: Mutex<Option<PermissionDecision>>,
    notify: Notify,
}

impl PermissionState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(None),
            notify: Notify::new(),
        })
    }

    /// Submit a decision. Wakes any waiters. If the decision's
    /// `tool_call_id` does not match the most recent request, it is stored
    /// so the matching waiter can still consume it.
    pub async fn resolve(&self, decision: PermissionDecision) {
        let mut guard = self.inner.lock().await;
        *guard = Some(decision);
        self.notify.notify_one();
    }

    /// Wait for a decision that matches `expected_tool_call_id`. Polls with
    /// `Notify` (no sleep loop). Returns `None` on mismatch after timeout
    /// or if the channel is closed.
    pub async fn wait_for(&self, expected_tool_call_id: &str) -> Option<PermissionDecision> {
        loop {
            {
                let mut guard = self.inner.lock().await;
                if let Some(decision) = guard.take() {
                    if decision_matches(&decision, expected_tool_call_id) {
                        return Some(decision);
                    }
                    // Stale decision for a different call. Re-store and keep
                    // waiting — caller will time out at the request layer.
                    *guard = Some(decision);
                }
            }
            self.notify.notified().await;
        }
    }

    /// Wait for any decision (no id match). Used by the agent loop as a
    /// fallback. The caller is responsible for matching the id.
    pub async fn wait(&self) -> Option<PermissionDecision> {
        loop {
            {
                let mut guard = self.inner.lock().await;
                if let Some(decision) = guard.take() {
                    return Some(decision);
                }
            }
            self.notify.notified().await;
        }
    }

    /// Clear any pending decision (used on interrupt / agent end).
    pub async fn clear(&self) {
        let mut guard = self.inner.lock().await;
        *guard = None;
    }
}

fn decision_matches(decision: &PermissionDecision, tool_call_id: &str) -> bool {
    let id = match decision {
        PermissionDecision::Allow { tool_call_id, .. }
        | PermissionDecision::AllowAlways { tool_call_id, .. }
        | PermissionDecision::Skip { tool_call_id, .. }
        | PermissionDecision::Deny { tool_call_id, .. } => tool_call_id,
    };
    id == tool_call_id
}
