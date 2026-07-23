//! Minimal permission infrastructure — bypass all by default (pi/Grok/Kimi Code style).
//!
//! The policy engine has been removed. All tool calls are allowed immediately.
//! The TUI approval dialog (`update/permission_dialog.rs`) is preserved for UX
//! but never shown since `PermissionGate::evaluate` always returns `Allow`.
//!
//! ## What was removed
//!
//! - `PermissionManager` + policy chain (`PermissionPolicy`, `BypassAllPolicy`,
//!   `BlockWriteToolsPolicy`, `AcceptEditsPolicy`, `AutoApprove`, `FileAccessAsk`,
//!   `ReadOnlyToolApprove`, `SensitivePathBlocklist`, `GitTrackedWriteApprove`,
//!   `DefaultToolApprove`)
//! - `PermissionMode` enum and `parse_permission_mode()`
//! - `PermissionSet`, `PermissionRule`, `PermissionScope`, `PermissionSetPolicy`
//! - `actors/permission/` ractor actor
//!
//! ## What is kept
//!
//! - `PermissionAction` / `PermissionResult` — kept for API compatibility
//! - `PermissionContext` — kept for API compatibility
//! - `ApprovalSink` trait + `AutoAllowSink` / `TuiApprovalSink` / `DenyAllSink` / `ScriptedSink`
//! - `PermissionGate` — now a simple pass-through to the sink
//! - `is_read_only_tool()` — still used by other code
//! - `is_sensitive_path()` — kept for reference, not enforced

use std::path::Path;
#[cfg(feature = "mcp")]
use rmcp::model::ToolAnnotations;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod gate;
mod sink;

pub use gate::PermissionGate;
pub use sink::{ApprovalSink, AutoAllowSink, DenyAllSink, ScriptedSink, TuiApprovalSink};

/// Build an approval sink based on yolo mode.
pub fn build_sink(yolo: bool) -> std::sync::Arc<dyn ApprovalSink> {
    if yolo {
        std::sync::Arc::new(AutoAllowSink)
    } else {
        std::sync::Arc::new(DenyAllSink)
    }
}

/// Permission action result — kept for API compatibility.
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

/// Result returned by a permission policy — kept for API compatibility.
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

/// Context passed to each policy during evaluation — kept for API compatibility.
#[derive(Debug, Clone)]
pub struct PermissionContext<'a> {
    pub tool: &'a str,
    pub path: Option<&'a Path>,
    pub input: Option<&'a Value>,
    pub cwd: Option<&'a Path>,
    #[cfg(feature = "mcp")]
    pub annotations: Option<ToolAnnotations>,
}

// ---------------------------------------------------------------------------
// Stubs kept for API compatibility (used by config parsing and subagents)
// ---------------------------------------------------------------------------

/// Permission mode — stub. All modes now bypass.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    Auto,
    DontAsk,
    BypassPermissions,
    Plan,
}

impl PermissionMode {
    pub fn bypasses_all(&self) -> bool {
        true
    }
    pub fn requires_plan(&self) -> bool {
        false
    }
    pub fn auto_approves_edits(&self) -> bool {
        true
    }
    pub fn auto_approves_safe(&self) -> bool {
        true
    }
}

/// Parse a permission mode string to enum variant.
pub fn parse_permission_mode(s: &str) -> PermissionMode {
    match s {
        "acceptEdits" => PermissionMode::AcceptEdits,
        "auto" => PermissionMode::Auto,
        "dontAsk" => PermissionMode::DontAsk,
        "bypassPermissions" => PermissionMode::BypassPermissions,
        "plan" => PermissionMode::Plan,
        _ => PermissionMode::Default,
    }
}

/// Permission rule — stub for config compatibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PermissionRule {
    pub action: PermissionAction,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

impl PermissionRule {
    pub fn new(action: PermissionAction, tool: impl Into<String>) -> Self {
        Self { action, tool: tool.into(), pattern: None }
    }
}

/// Permission scope — stub for config compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    #[default]
    User,
    Project,
    Session,
}

/// Permission set — stub for config compatibility.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    _private: (),
}

impl PermissionSet {
    pub fn new(_rules: Vec<PermissionRule>) -> Self {
        Self { _private: () }
    }
    pub fn default_rules() -> Self {
        Self { _private: () }
    }
    pub fn accept_edits_rules() -> Self {
        Self { _private: () }
    }
    pub fn dont_ask_rules() -> Self {
        Self { _private: () }
    }
    pub fn to_permission_set(&self) -> Self {
        Self { _private: () }
    }
}

/// Permission set policy — stub for config compatibility.
#[derive(Debug, Clone)]
pub struct PermissionSetPolicy;

impl PermissionSetPolicy {
    pub fn new(_rules: PermissionSet) -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Tools that are read-only (safe for auto-approval).
pub fn is_read_only_tool(tool: &str) -> bool {
    matches!(tool, "read_file" | "grep" | "find" | "list_dir" | "fetch_docs")
}
