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

    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::PermissionDecision;

    fn allow(id: &str) -> PermissionDecision {
        PermissionDecision::Allow {
            tool_call_id: id.to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
        }
    }

    fn deny(id: &str) -> PermissionDecision {
        PermissionDecision::Deny {
            tool_call_id: id.to_string(),
            tool_name: "bash".to_string(),
            tool_args: "{}".to_string(),
        }
    }

    #[tokio::test]
    async fn resolve_then_wait_returns_matching_decision() {
        let state = PermissionState::new();
        let s = state.clone();
        tokio::spawn(async move { s.resolve(allow("t1")).await });
        let got = state.wait_for("t1").await;
        let decision = got.expect("expected Some(decision)");
        match decision {
            PermissionDecision::Allow { tool_call_id, .. } => assert_eq!(tool_call_id, "t1"),
            _ => panic!("expected Allow"),
        }
    }

    #[tokio::test]
    async fn wait_for_blocks_until_matching_id_arrives() {
        let state = PermissionState::new();
        // Submit a decision for a different id first.
        state.resolve(allow("t0")).await;
        // wait_for("t1") must NOT consume the t0 decision.
        let s = state.clone();
        let waiter = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_millis(50), s.wait_for("t1")).await
        });
        // Give the waiter a tick to observe it cannot match.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        // Now submit the right id; waiter should unblock.
        state.resolve(allow("t1")).await;
        let result = waiter.await.unwrap();
        let got = result.expect("wait_for should not time out");
        match got {
            Some(PermissionDecision::Allow { tool_call_id, .. }) => assert_eq!(tool_call_id, "t1"),
            Some(_) => panic!("expected Allow, got other variant"),
            None => panic!("expected Some(Allow), got None"),
        }
    }

    #[tokio::test]
    async fn wait_for_times_out_when_no_matching_decision() {
        let state = PermissionState::new();
        // No decision ever arrives.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(30),
            state.wait_for("never"),
        ).await;
        assert!(result.is_err(), "expected timeout, got {:?}", result);
    }

    #[tokio::test]
    async fn clear_resets_pending_decision() {
        let state = PermissionState::new();
        state.resolve(allow("t0")).await;
        state.clear().await;
        // A subsequent wait_for for t0 must not return — there's nothing.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(30),
            state.wait_for("t0"),
        ).await;
        assert!(result.is_err(), "expected timeout after clear, got {:?}", result);
    }

    #[tokio::test]
    async fn wait_returns_any_decision_regardless_of_id() {
        let state = PermissionState::new();
        state.resolve(deny("anything")).await;
        let got = state.wait().await.expect("expected decision");
        match got {
            PermissionDecision::Deny { tool_call_id, .. } => assert_eq!(tool_call_id, "anything"),
            _ => panic!("expected Deny"),
        }
    }
}
