//! Permission policy chain.
//!
//! Evaluates a chain of `PermissionPolicy` implementations in order; the first
//! matching policy wins. Legacy `PermissionSet`/`PermissionRule` rulesets are
//! preserved and re-exported for compatibility.

use std::path::Path;

use async_trait::async_trait;
use glob::Pattern;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod default_tool_approve;
pub mod file_access_ask;
pub mod git_tracked_write;
mod rules;
mod sink;

pub use default_tool_approve::DefaultToolApprove;
pub use file_access_ask::FileAccessAsk;
pub use git_tracked_write::GitTrackedWriteApprove;
pub use rules::{PermissionRule, PermissionSet};
pub use sink::{ApprovalSink, AutoAllowSink, DenyAllSink, ScriptedSink, TuiApprovalSink};

#[cfg(test)]
mod tests;

/// Permission action result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

/// Result returned by a permission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionResult {
    Allow,
    Ask,
    Deny,
}

impl From<PermissionResult> for PermissionAction {
    fn from(result: PermissionResult) -> Self {
        match result {
            PermissionResult::Allow => PermissionAction::Allow,
            PermissionResult::Ask => PermissionAction::Ask,
            PermissionResult::Deny => PermissionAction::Deny,
        }
    }
}

/// Global permission mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// Auto-approve everything.
    Yolo,
    /// Always ask.
    Manual,
    /// Use the policy chain.
    #[default]
    Auto,
}

/// Context passed to each policy during evaluation.
#[derive(Debug, Clone, Copy)]
pub struct PermissionContext<'a> {
    pub tool: &'a str,
    pub path: Option<&'a Path>,
    pub input: Option<&'a Value>,
    pub cwd: Option<&'a Path>,
}

/// A single permission policy in the chain.
#[async_trait]
pub trait PermissionPolicy: Send + Sync {
    fn name(&self) -> &str;
    fn matches(&self, ctx: &PermissionContext<'_>) -> bool;
    async fn evaluate(&self, ctx: &PermissionContext<'_>) -> Option<PermissionResult>;
}

/// Evaluates policies in order; first matching policy wins.
#[derive(Default)]
pub struct PermissionManager {
    policies: Vec<Box<dyn PermissionPolicy>>,
    mode: PermissionMode,
}

impl PermissionManager {
    pub fn new(mode: PermissionMode) -> Self {
        Self {
            policies: Vec::new(),
            mode,
        }
    }

    pub fn with_policies(mut self, policies: Vec<Box<dyn PermissionPolicy>>) -> Self {
        self.policies = policies;
        self
    }

    pub fn add_policy(&mut self, policy: Box<dyn PermissionPolicy>) {
        self.policies.push(policy);
    }

    pub fn mode(&self) -> PermissionMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }

    /// Evaluate the context against the policy chain.
    pub async fn evaluate(&self, ctx: &PermissionContext<'_>) -> PermissionResult {
        match self.mode {
            PermissionMode::Yolo => return PermissionResult::Allow,
            PermissionMode::Manual => return PermissionResult::Ask,
            PermissionMode::Auto => {}
        }
        for policy in &self.policies {
            if policy.matches(ctx) {
                if let Some(result) = policy.evaluate(ctx).await {
                    return result;
                }
            }
        }
        PermissionResult::Ask
    }
}

/// Sensitive path patterns that are always denied.
pub fn is_sensitive_path(path: &str) -> bool {
    let sensitive = [
        "**/.env",
        ".env",
        "**/.ssh/*",
        ".ssh/*",
        "**/.aws/*",
        ".aws/*",
        "**/.git/config",
    ];
    sensitive
        .iter()
        .any(|p| Pattern::new(p).is_ok_and(|pat| pat.matches(path)))
}

/// Tools that are read-only (safe for auto-approval).
pub fn is_read_only_tool(tool: &str) -> bool {
    matches!(
        tool,
        "read_file" | "grep" | "find" | "list_dir" | "fetch_docs"
    )
}
