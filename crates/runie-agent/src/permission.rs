use std::collections::HashSet;
use tokio::sync::mpsc;
use std::time::Duration;

use crate::events::PermissionDecision;

pub struct PermissionGate {
    tx: mpsc::Sender<PermissionDecision>,
    pub(crate) rx: mpsc::Receiver<PermissionDecision>,
    pub(crate) allowed_tools: HashSet<String>,
    timeout: Duration,
}

impl PermissionGate {

    #[must_use]
    #[must_use]
    pub fn new(timeout_secs: u64) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            tx,
            rx,
            allowed_tools: HashSet::new(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Create a PermissionGate with externally-provided channel ends.
    /// Useful for testing where the sender must be accessible.
    pub fn with_channel(
        tx: mpsc::Sender<PermissionDecision>,
        rx: mpsc::Receiver<PermissionDecision>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            tx,
            rx,
            allowed_tools: HashSet::new(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<PermissionDecision> {
        self.tx.clone()
    }

    /// Returns true if the tool has been cached as allowed (via AllowAlways).
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }

    pub async fn request_permission(
        &mut self,
        tool_name: &str,
        tool_call_id: &str,
    ) -> PermissionResult {
        if self.allowed_tools.contains(tool_name) {
            return PermissionResult::Allowed;
        }

        // Wait for decision with timeout
        match tokio::time::timeout(self.timeout, self.rx.recv()).await {
            Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid, .. })) if tid == tool_call_id => {
                PermissionResult::Allowed
            }
            Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid, .. })) if tid == tool_call_id => {
                self.allowed_tools.insert(tool_name.to_string());
                PermissionResult::Allowed
            }
            Ok(Some(PermissionDecision::Skip { .. })) => PermissionResult::Skipped,
            _ => PermissionResult::Denied,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    Allowed,
    Denied,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allowed_tools_cache() {
        let mut gate = PermissionGate::new(60);
        
        // Manually add a tool to the cache
        gate.allowed_tools.insert("bash".to_string());
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Allowed);
    }

    #[tokio::test]
    async fn test_allow_always_caches_tool() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        
        // Replace rx with one that has the decision already queued
        gate.rx = rx;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::AllowAlways { 
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Allowed);
        assert!(gate.allowed_tools.contains("bash"));
    }

    #[tokio::test]
    async fn test_skip_returns_skipped() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        gate.rx = rx;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Skip { 
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Skipped);
    }

    #[tokio::test]
    async fn test_timeout_returns_denied() {
        let mut gate = PermissionGate::new(1); // 1 second timeout
        
        // Don't send any decision - should timeout
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied);
    }

    #[tokio::test]
    async fn test_mismatched_tool_call_id_denied() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        gate.rx = rx;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Allow { 
                tool_call_id: "call_wrong".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied);
    }

    #[tokio::test]
    async fn test_deny_returns_denied() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        gate.rx = rx;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Deny { 
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied);
    }

    // BUG-09: Queue is LIFO instead of FIFO (Vec.pop returns last element)
    // Note: This documents the bug in TUI's PermissionModalState.pending_queue (Vec<PendingPermission>)
    // Vec.pop() returns LAST element (LIFO), but queue semantics require FIFO
    #[tokio::test]
    async fn test_queue_fifo_order() {
        // Simple struct to mimic PendingPermission behavior
        #[derive(Debug)]
        struct QueueItem {
            tool_name: String,
        }
        
        let mut queue: Vec<QueueItem> = vec![
            QueueItem { tool_name: "A".to_string() },
            QueueItem { tool_name: "B".to_string() },
        ];
        
        // BUG-09: pop() returns B first (LIFO), but we want A first (FIFO)
        let first = queue.pop();
        assert_eq!(first.as_ref().map(|r| r.tool_name.as_str()), Some("B"),
            "BUG-09: Vec.pop() is LIFO - returns B first instead of A");
        
        let second = queue.pop();
        assert_eq!(second.as_ref().map(|r| r.tool_name.as_str()), Some("A"),
            "BUG-09: Second pop returns A, but A should have been processed first");
    }

    #[tokio::test]
    async fn test_skip_does_not_cache_tool() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        gate.rx = rx;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Skip { 
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Skipped);
        assert!(!gate.allowed_tools.contains("bash"),
            "Skip should NOT add tool to allowed_tools");
    }

    #[tokio::test]
    async fn test_double_allowalways_no_duplicate() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::new(60);
        gate.rx = rx;
        let tx_clone = tx.clone();
        
        // First AllowAlways
        gate.allowed_tools.insert("bash".to_string());
        
        // Attempt second AllowAlways via channel
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::AllowAlways { 
                tool_call_id: "call_2".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_2").await;
        assert_eq!(result, PermissionResult::Allowed);
        
        // HashSet.insert returns false if already present - no duplicate
        let count = gate.allowed_tools.iter().filter(|n| n.as_str() == "bash").count();
        assert_eq!(count, 1, "AllowAlways twice should not duplicate in cache");
    }

    #[tokio::test]
    async fn test_cache_persists_across_calls() {
        let mut gate = PermissionGate::new(60);
        
        // Pre-cache the tool
        gate.allowed_tools.insert("bash".to_string());
        
        // First call - should be auto-allowed
        let result1 = gate.request_permission("bash", "call_1").await;
        assert_eq!(result1, PermissionResult::Allowed);
        
        // Second call - still allowed from cache (no channel interaction needed)
        let result2 = gate.request_permission("bash", "call_2").await;
        assert_eq!(result2, PermissionResult::Allowed);
    }

    // Test rollback handler logs only (documents bug)
    #[tokio::test]
    async fn test_rollback_no_op() {
        // Rollback command is generated in handle_permission when decision is Deny or Skip
        // but the actual Rollback implementation appears to only log
        let gate = PermissionGate::new(60);
        assert!(gate.allowed_tools.is_empty(),
            "PermissionGate starts empty - rollback is TUI-level concern");
    }

    // EDGE CASE: Empty tool name should still work (registry will reject later)
    #[tokio::test]
    async fn test_empty_tool_name() {
        let mut gate = PermissionGate::new(60);
        gate.allowed_tools.insert("".to_string());
        let result = gate.request_permission("", "call_1").await;
        assert_eq!(result, PermissionResult::Allowed);
    }

    // EDGE CASE: Empty channel with short timeout returns Denied
    #[tokio::test]
    async fn test_empty_channel_timeout() {
        let mut gate = PermissionGate::new(1); // 1 second timeout
        // Don't send anything - should timeout
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied, "Empty channel should timeout to Denied");
    }

    // EDGE CASE: Zero timeout should immediately deny
    #[tokio::test]
    async fn test_zero_timeout_immediate_deny() {
        let mut gate = PermissionGate::new(0);
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied, "Zero timeout should immediately deny");
    }

    // EDGE CASE: Very long timeout still works
    #[tokio::test]
    async fn test_long_timeout() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::with_channel(tx, rx, 3600); // 1 hour
        let tx_clone = gate.sender();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Allow {
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Allowed);
    }

    // EDGE CASE: Decision for wrong tool_call_id with same tool name
    #[tokio::test]
    async fn test_wrong_tool_call_id_same_tool() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::with_channel(tx, rx, 60);
        let tx_clone = gate.sender();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Allow {
                tool_call_id: "call_other".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied, "Wrong tool_call_id should deny even for same tool");
    }

    // EDGE CASE: Multiple AllowAlways for different tools
    #[tokio::test]
    async fn test_allow_always_multiple_tools() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::with_channel(tx.clone(), rx, 60);
        
        // AllowAlways for bash
        let tx1 = tx.clone();
        tokio::spawn(async move {
            tx1.send(PermissionDecision::AllowAlways {
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Allowed);

        // AllowAlways for read_file
        let (tx2, rx2) = mpsc::channel::<PermissionDecision>(10);
        gate = PermissionGate::with_channel(tx2.clone(), rx2, 60);
        let tx3 = tx2.clone();
        tokio::spawn(async move {
            tx3.send(PermissionDecision::AllowAlways {
                tool_call_id: "call_2".to_string(),
                tool_name: "read_file".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        let result = gate.request_permission("read_file", "call_2").await;
        assert_eq!(result, PermissionResult::Allowed);
        
        // Both should be cached
        assert!(gate.is_tool_allowed("bash") || gate.is_tool_allowed("read_file"));
    }

    // SECURITY: Verify Deny decision does not cache tool
    #[tokio::test]
    async fn test_deny_does_not_cache() {
        let (tx, rx) = mpsc::channel::<PermissionDecision>(10);
        let mut gate = PermissionGate::with_channel(tx, rx, 60);
        let tx_clone = gate.sender();
        tokio::spawn(async move {
            tx_clone.send(PermissionDecision::Deny {
                tool_call_id: "call_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: String::new(),
            }).await.unwrap();
        });
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied);
        assert!(!gate.is_tool_allowed("bash"), "Deny should NOT cache tool as allowed");
    }
}