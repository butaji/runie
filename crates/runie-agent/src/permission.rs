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
            Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid })) if tid == tool_call_id => {
                PermissionResult::Allowed
            }
            Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid })) if tid == tool_call_id => {
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
                tool_call_id: "call_1".to_string() 
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
                tool_call_id: "call_1".to_string() 
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
                tool_call_id: "call_wrong".to_string() 
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
                tool_call_id: "call_1".to_string() 
            }).await.unwrap();
        });
        
        let result = gate.request_permission("bash", "call_1").await;
        assert_eq!(result, PermissionResult::Denied);
    }
}