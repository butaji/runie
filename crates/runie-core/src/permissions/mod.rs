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
use rmcp::model::ToolAnnotations;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod default_tool_approve;
pub mod file_access_ask;
pub mod gate;
pub mod git_tracked_write;
pub mod rules;
mod sink;

pub use default_tool_approve::DefaultToolApprove;
pub use file_access_ask::FileAccessAsk;
pub use gate::PermissionGate;
pub use git_tracked_write::GitTrackedWriteApprove;
pub use rules::{PermissionRule, PermissionScope, PermissionSet, PermissionSetPolicy};
pub use sink::{ApprovalSink, AutoAllowSink, DenyAllSink, ScriptedSink, TuiApprovalSink};

#[cfg(test)]
mod tests;

/// Permission action result.
///
/// This is the canonical enum for permission decisions throughout the codebase.
/// The protocol's `ApprovalDecision` (Allow/Deny only) is converted to this type
/// at the protocol/core boundary via `From<crate::proto::op::ApprovalDecision>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
    strum::EnumString,
)]
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

/// Parse a permission mode from a string, supporting both canonical snake_case
/// names and legacy camelCase names from subagent YAML frontmatter.
pub fn parse_permission_mode(s: &str) -> PermissionMode {
    // Try canonical FromStr first (snake_case via strum).
    if let Ok(mode) = s.parse::<PermissionMode>() {
        return mode;
    }
    // Fall back to legacy camelCase/frontmatter names.
    match s {
        "acceptEdits" => PermissionMode::AcceptEdits,
        "auto" => PermissionMode::Auto,
        "dontAsk" => PermissionMode::DontAsk,
        "bypassPermissions" => PermissionMode::BypassPermissions,
        "plan" => PermissionMode::Plan,
        _ => PermissionMode::Default,
    }
}

/// Context passed to each policy during evaluation.
#[derive(Debug, Clone)]
pub struct PermissionContext<'a> {
    pub tool: &'a str,
    pub path: Option<&'a Path>,
    pub input: Option<&'a Value>,
    pub cwd: Option<&'a Path>,
    /// MCP tool annotations for the requested tool, if known.
    /// Populated by `PermissionActor` from `runie_core::tool::annotations`.
    pub annotations: Option<ToolAnnotations>,
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
    /// Create a new manager with the default policy chain for the given mode.
    pub fn new(mode: PermissionMode) -> Self {
        let policies = Self::build_policies(mode);
        Self { policies }
    }

    /// Build the default policy chain for a permission mode.
    fn build_policies(mode: PermissionMode) -> Vec<Box<dyn PermissionPolicy>> {
        match mode {
            PermissionMode::BypassPermissions => {
                // Auto-approve everything.
                vec![Box::new(BypassAllPolicy)]
            }
            PermissionMode::Plan => {
                // Block all write tools until plan is approved.
                vec![Box::new(BlockWriteToolsPolicy)]
            }
            PermissionMode::Auto => {
                // Auto-approve safe tools, ask for others.
                vec![
                    Box::new(DefaultToolApprove::new()),
                    Box::new(FileAccessAsk::new()),
                ]
            }
            PermissionMode::AcceptEdits => {
                // Auto-approve read and write, ask for bash.
                vec![
                    Box::new(DefaultToolApprove::new()),
                    Box::new(AcceptEditsPolicy),
                ]
            }
            PermissionMode::DontAsk => {
                // Allow all unless explicit deny rule exists (handled by PermissionSet).
                vec![]
            }
            PermissionMode::Default => {
                // Ask for all operations that match file access outside cwd.
                vec![Box::new(FileAccessAsk::new())]
            }
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

// Policy implementations for PermissionMode-driven chains.
// These policies are used by PermissionManager when built with a PermissionMode.

/// Auto-approve all operations (BypassPermissions mode).
struct BypassAllPolicy;

#[async_trait]
impl PermissionPolicy for BypassAllPolicy {
    fn name(&self) -> &str {
        "bypass_all"
    }

    fn matches(&self, _ctx: &PermissionContext<'_>) -> bool {
        true
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Allow)
    }
}

/// Block write tools until plan is approved (Plan mode).
struct BlockWriteToolsPolicy;

impl BlockWriteToolsPolicy {
    fn is_write_tool(tool: &str) -> bool {
        matches!(
            tool,
            "write_file" | "edit_file" | "bash" | "delete_file" | "create_directory"
        )
    }
}

#[async_trait]
impl PermissionPolicy for BlockWriteToolsPolicy {
    fn name(&self) -> &str {
        "block_write_tools"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        Self::is_write_tool(ctx.tool)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Ask)
    }
}

/// Auto-approve file edits (AcceptEdits mode).
struct AcceptEditsPolicy;

impl AcceptEditsPolicy {
    fn is_edit_tool(tool: &str) -> bool {
        matches!(tool, "write_file" | "edit_file")
    }
}

#[async_trait]
impl PermissionPolicy for AcceptEditsPolicy {
    fn name(&self) -> &str {
        "accept_edits"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        Self::is_edit_tool(ctx.tool)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Allow)
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
    sensitive.iter().any(|p| glob_matches(p, path))
}

/// Match a glob pattern against a string using the `glob` crate.
fn glob_matches(pattern: &str, name: &str) -> bool {
    use glob::Pattern;
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
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
