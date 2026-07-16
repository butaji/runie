//! Approval sinks for permission prompts.

use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::Value;

use super::PermissionAction;

/// Approval sink for permission prompts.
#[async_trait]
pub trait ApprovalSink: Send + Sync {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction;
}

/// Always allow — for headless/trusted mode.
pub struct AutoAllowSink;
#[async_trait]
impl ApprovalSink for AutoAllowSink {
    async fn ask(&self, _tool: &str, _input: &Value) -> PermissionAction {
        PermissionAction::Allow
    }
}

/// Always ask — for TUI mode.
pub struct TuiApprovalSink;
#[async_trait]
impl ApprovalSink for TuiApprovalSink {
    async fn ask(&self, _tool: &str, _input: &Value) -> PermissionAction {
        PermissionAction::Ask
    }
}

/// Always deny — safe default for headless/server modes.
pub struct DenyAllSink;
#[async_trait]
impl ApprovalSink for DenyAllSink {
    async fn ask(&self, _tool: &str, _input: &Value) -> PermissionAction {
        PermissionAction::Deny
    }
}

/// Scripted sink for tests.
#[derive(Default)]
pub struct ScriptedSink {
    decisions: RwLock<Vec<(String, PermissionAction)>>,
}

impl ScriptedSink {
    pub fn new() -> Self {
        Self {
            decisions: RwLock::new(Vec::new()),
        }
    }
    pub fn add_decision(&self, tool: impl Into<String>, action: PermissionAction) {
        self.decisions.write().push((tool.into(), action));
    }
}

#[async_trait]
impl ApprovalSink for ScriptedSink {
    async fn ask(&self, tool: &str, _input: &Value) -> PermissionAction {
        let d = self.decisions.read();
        for (t, a) in d.iter().rev() {
            if t == tool {
                return *a;
            }
        }
        PermissionAction::Ask
    }
}
