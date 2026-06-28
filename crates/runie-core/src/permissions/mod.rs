//! Permission policy chain with declarative rule evaluation.
//!
//! Evaluates a chain of `PermissionPolicy` implementations in order; the first
//! matching policy wins. Legacy `PermissionSet`/`PermissionRule` rulesets are
//! preserved and re-exported for compatibility.
//!
//! ## Permission Modes
//!
//! | Mode | Behavior |
//! |------|----------|
//! | `default` | Apply rules; ask when no rule matches |
//! | `acceptEdits` | Auto-accept file edits; ask for shell commands |
//! | `auto` | Auto-approve safe operations; ask for risky ones |
//! | `dontAsk` | Approve unless a deny rule matches |
//! | `bypassPermissions` | Approve everything (dangerous) |
//! | `plan` | Block write tools until a plan is approved |

use std::path::Path;

use async_trait::async_trait;
use crate::glob::matches;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod approval_registry;
pub mod default_tool_approve;
pub mod file_access_ask;
pub mod gate;
pub mod git_tracked_write;
pub mod rules;
mod sink;

pub use approval_registry::ApprovalRegistry;
pub use default_tool_approve::DefaultToolApprove;
pub use file_access_ask::FileAccessAsk;
pub use gate::PermissionGate;
pub use git_tracked_write::GitTrackedWriteApprove;
pub use rules::{PermissionRule, PermissionScope, PermissionSet};
pub use sink::{ApprovalSink, AutoAllowSink, DenyAllSink, ScriptedSink, TuiApprovalSink};

#[cfg(test)]
mod tests;

/// Permission action result.
///
/// This is the canonical enum for permission decisions throughout the codebase.
/// The protocol's `ApprovalDecision` (Allow/Deny only) is converted to this type
/// at the protocol/core boundary via `From<crate::proto::op::ApprovalDecision>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

impl From<crate::proto::op::ApprovalDecision> for PermissionAction {
    fn from(decision: crate::proto::op::ApprovalDecision) -> Self {
        match decision {
            crate::proto::op::ApprovalDecision::Allow => PermissionAction::Allow,
            crate::proto::op::ApprovalDecision::Deny => PermissionAction::Deny,
        }
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// Apply rules; ask when no rule matches.
    #[default]
    Default,
    /// Auto-accept file edits; ask for shell commands.
    AcceptEdits,
    /// Auto-approve safe operations; ask for risky ones.
    Auto,
    /// Approve unless a deny rule matches.
    DontAsk,
    /// Approve everything (dangerous).
    BypassPermissions,
    /// Block write tools until a plan is approved.
    Plan,
}

impl PermissionMode {
    /// Returns true if this mode bypasses all permission checks.
    pub fn bypasses_all(&self) -> bool {
        matches!(self, PermissionMode::BypassPermissions)
    }

    /// Returns true if this mode blocks write tools until a plan is approved.
    pub fn requires_plan(&self) -> bool {
        matches!(self, PermissionMode::Plan)
    }

    /// Returns true if this mode auto-approves file edits.
    pub fn auto_approves_edits(&self) -> bool {
        matches!(self, PermissionMode::AcceptEdits)
    }

    /// Returns true if this mode auto-approves safe tools.
    pub fn auto_approves_safe(&self) -> bool {
        matches!(self, PermissionMode::Auto | PermissionMode::AcceptEdits)
    }
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
}

impl PermissionManager {
    pub fn new(_mode: PermissionMode) -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn with_policies(mut self, policies: Vec<Box<dyn PermissionPolicy>>) -> Self {
        self.policies = policies;
        self
    }

    pub fn add_policy(&mut self, policy: Box<dyn PermissionPolicy>) {
        self.policies.push(policy);
    }

    /// Evaluate the context against the policy chain.
    pub async fn evaluate(&self, ctx: &PermissionContext<'_>) -> PermissionResult {
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
    sensitive.iter().any(|p| matches(p, path))
}

/// Build an approval sink based on yolo mode.
///
/// When `yolo` is true, returns an auto-allow sink that approves all tool calls.
/// When `yolo` is false, returns a deny-all sink that blocks all tool calls.
pub fn build_sink(yolo: bool) -> std::sync::Arc<dyn ApprovalSink> {
    if yolo {
        std::sync::Arc::new(AutoAllowSink)
    } else {
        std::sync::Arc::new(DenyAllSink)
    }
}

/// Tools that are read-only (safe for auto-approval).
pub fn is_read_only_tool(tool: &str) -> bool {
    matches!(
        tool,
        "read_file" | "grep" | "find" | "list_dir" | "fetch_docs"
    )
}
