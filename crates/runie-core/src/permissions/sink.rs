//! Approval sinks for permission prompts.

use async_trait::async_trait;
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

/// Scripted sink for tests.
pub struct ScriptedSink {
    decisions: std::sync::RwLock<Vec<(String, PermissionAction)>>,
}

impl Default for ScriptedSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptedSink {
    pub fn new() -> Self {
        Self {
            decisions: std::sync::RwLock::new(Vec::new()),
        }
    }
    pub fn add_decision(&self, tool: impl Into<String>, action: PermissionAction) {
        if let Ok(mut d) = self.decisions.write() {
            d.push((tool.into(), action));
        }
    }
}

#[async_trait]
impl ApprovalSink for ScriptedSink {
    async fn ask(&self, tool: &str, _input: &Value) -> PermissionAction {
        if let Ok(d) = self.decisions.read() {
            for (t, a) in d.iter().rev() {
                if t == tool {
                    return *a;
                }
            }
        }
        PermissionAction::Ask
    }
}
