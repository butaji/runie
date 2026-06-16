//! Permission rulesets with wildcard matching and ApprovalSink trait.
//!
//! Evaluation: last-match wins. Sensitive paths are always denied.

use glob::Pattern;
use serde::{Deserialize, Serialize};

/// Permission action result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

/// A single permission rule with glob patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Glob pattern for tool name, e.g. "bash", "read_*", "*".
    pub tool_pattern: String,
    /// Optional glob pattern for file paths.
    pub path_pattern: Option<String>,
    /// Action when rule matches.
    pub action: PermissionAction,
}

impl PermissionRule {
    fn matches_tool(&self, tool: &str) -> bool {
        Pattern::new(&self.tool_pattern).map_or(false, |p| p.matches(tool))
    }
    fn matches_path(&self, path: &str) -> bool {
        match &self.path_pattern {
            Some(p) => Pattern::new(p).map_or(false, |pat| pat.matches(path)),
            None => true,
        }
    }
    pub fn matches(&self, tool: &str, path: Option<&str>) -> bool {
        if !self.matches_tool(tool) { return false; }
        match path {
            Some(p) => self.matches_path(p),
            None => self.path_pattern.is_none(),
        }
    }
}

/// A set of permission rules (last-match wins).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet { rules: Vec<PermissionRule> }

impl PermissionSet {
    pub fn new(rules: Vec<PermissionRule>) -> Self { Self { rules } }

    /// Evaluate tool+path against rules (last-match wins).
    pub fn evaluate(&self, tool: &str, path: Option<&str>) -> PermissionAction {
        let mut result = PermissionAction::Ask;
        for rule in &self.rules {
            if rule.matches(tool, path) { result = rule.action; }
        }
        result
    }

    /// Evaluate with the built-in sensitive-path denylist applied first.
    pub fn effective_action(&self, tool: &str, path: Option<&str>) -> PermissionAction {
        if let Some(p) = path {
            if is_sensitive_path(p) {
                return PermissionAction::Deny;
            }
        }
        self.evaluate(tool, path)
    }

    /// Default rules matching the agent's historical behavior:
    /// read-only tools are allowed, write/edit/bash ask, and sensitive paths
    /// are denied via `effective_action`.
    pub fn default_rules() -> Self {
        Self::new(vec![
            PermissionRule { tool_pattern: "read_file".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "list_dir".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "grep".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "find".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "fetch_docs".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "write_file".into(), path_pattern: None, action: PermissionAction::Ask },
            PermissionRule { tool_pattern: "edit_file".into(), path_pattern: None, action: PermissionAction::Ask },
            PermissionRule { tool_pattern: "bash".into(), path_pattern: None, action: PermissionAction::Ask },
        ])
    }

    pub fn rules(&self) -> &[PermissionRule] { &self.rules }
}

/// Sensitive path patterns that are always denied.
pub fn is_sensitive_path(path: &str) -> bool {
    let sensitive = ["**/.env", ".env", "**/.ssh/*", ".ssh/*", "**/.aws/*", ".aws/*", "**/.git/config"];
    sensitive.iter().any(|p| Pattern::new(p).map_or(false, |pat| pat.matches(path)))
}

// ---------------------------------------------------------------------------
// ApprovalSink trait
// ---------------------------------------------------------------------------

use async_trait::async_trait;
use serde_json::Value;

/// Approval sink for permission prompts.
#[async_trait]
pub trait ApprovalSink: Send + Sync {
    async fn ask(&self, tool: &str, input: &Value) -> PermissionAction;
}

/// Always allow — for headless/trusted mode.
pub struct AutoAllowSink;
#[async_trait]
impl ApprovalSink for AutoAllowSink {
    async fn ask(&self, _tool: &str, _input: &Value) -> PermissionAction { PermissionAction::Allow }
}

/// Always ask — for TUI mode.
pub struct TuiApprovalSink;
#[async_trait]
impl ApprovalSink for TuiApprovalSink {
    async fn ask(&self, _tool: &str, _input: &Value) -> PermissionAction { PermissionAction::Ask }
}

/// Scripted sink for tests.
pub struct ScriptedSink { decisions: std::sync::RwLock<Vec<(String, PermissionAction)>> }
impl ScriptedSink {
    pub fn new() -> Self { Self { decisions: std::sync::RwLock::new(Vec::new()) } }
    pub fn add_decision(&self, tool: impl Into<String>, action: PermissionAction) {
        if let Ok(mut d) = self.decisions.write() { d.push((tool.into(), action)); }
    }
}
#[async_trait]
impl ApprovalSink for ScriptedSink {
    async fn ask(&self, tool: &str, _input: &Value) -> PermissionAction {
        if let Ok(d) = self.decisions.read() {
            for (t, a) in d.iter().rev() { if t == tool { return *a; } }
        }
        PermissionAction::Ask
    }
}

// ---------------------------------------------------------------------------
// Read-only tool classification
// ---------------------------------------------------------------------------

/// Tools that are read-only (safe for auto-approval).
pub fn is_read_only_tool(tool: &str) -> bool {
    matches!(tool, "read_file" | "grep" | "find" | "list_dir" | "fetch_docs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_rule_matches_tool() {
        let rules = PermissionSet::new(vec![
            PermissionRule { tool_pattern: "*".into(), path_pattern: None, action: PermissionAction::Allow },
        ]);
        assert_eq!(rules.evaluate("bash", None), PermissionAction::Allow);
        assert_eq!(rules.evaluate("read_file", None), PermissionAction::Allow);
    }

    #[test]
    fn path_rule_matches_file() {
        let rules = PermissionSet::new(vec![
            PermissionRule { tool_pattern: "read_file".into(), path_pattern: Some("src/**".into()), action: PermissionAction::Allow },
        ]);
        assert_eq!(rules.evaluate("read_file", Some("src/main.rs")), PermissionAction::Allow);
        assert_eq!(rules.evaluate("read_file", Some("other/file.rs")), PermissionAction::Ask);
    }

    #[test]
    fn last_rule_wins() {
        let rules = PermissionSet::new(vec![
            PermissionRule { tool_pattern: "bash".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "bash".into(), path_pattern: None, action: PermissionAction::Deny },
        ]);
        assert_eq!(rules.evaluate("bash", None), PermissionAction::Deny);
    }

    #[test]
    fn sensitive_path_denied() {
        assert!(is_sensitive_path("/home/user/.ssh/id_rsa"));
        assert!(is_sensitive_path("/project/.env"));
        assert!(!is_sensitive_path("/project/src/main.rs"));
    }

    #[test]
    fn read_only_tool_classification() {
        assert!(is_read_only_tool("read_file"));
        assert!(is_read_only_tool("grep"));
        assert!(!is_read_only_tool("bash"));
        assert!(!is_read_only_tool("write_file"));
    }

    #[tokio::test]
    async fn auto_allow_sink_always_allows() {
        let sink = AutoAllowSink;
        let action = sink.ask("bash", &serde_json::json!({"command": "ls"})).await;
        assert_eq!(action, PermissionAction::Allow);
    }

    #[tokio::test]
    async fn scripted_sink_returns_decisions() {
        let sink = ScriptedSink::new();
        sink.add_decision("bash", PermissionAction::Allow);
        sink.add_decision("write_file", PermissionAction::Deny);
        assert_eq!(sink.ask("bash", &Value::Null).await, PermissionAction::Allow);
        assert_eq!(sink.ask("write_file", &Value::Null).await, PermissionAction::Deny);
        assert_eq!(sink.ask("read_file", &Value::Null).await, PermissionAction::Ask); // default
    }

    #[test]
    fn permission_set_default_is_ask() {
        let rules = PermissionSet::default();
        assert_eq!(rules.evaluate("bash", None), PermissionAction::Ask);
    }

    #[test]
    fn permission_set_evaluates_rules() {
        // Last-match wins: list the default/catch-all first, then overrides.
        let rules = PermissionSet::new(vec![
            PermissionRule { tool_pattern: "*".into(), path_pattern: None, action: PermissionAction::Deny },
            PermissionRule { tool_pattern: "read_*".into(), path_pattern: None, action: PermissionAction::Allow },
            PermissionRule { tool_pattern: "bash".into(), path_pattern: None, action: PermissionAction::Ask },
        ]);
        assert_eq!(rules.evaluate("read_file", None), PermissionAction::Allow);
        assert_eq!(rules.evaluate("bash", None), PermissionAction::Ask);
        assert_eq!(rules.evaluate("unknown", None), PermissionAction::Deny);
    }

    #[test]
    fn default_rules_read_only_allowed_write_asks() {
        let rules = PermissionSet::default_rules();
        assert_eq!(rules.effective_action("read_file", None), PermissionAction::Allow);
        assert_eq!(rules.effective_action("list_dir", None), PermissionAction::Allow);
        assert_eq!(rules.effective_action("grep", None), PermissionAction::Allow);
        assert_eq!(rules.effective_action("find", None), PermissionAction::Allow);
        assert_eq!(rules.effective_action("fetch_docs", None), PermissionAction::Allow);
        assert_eq!(rules.effective_action("write_file", None), PermissionAction::Ask);
        assert_eq!(rules.effective_action("edit_file", None), PermissionAction::Ask);
        assert_eq!(rules.effective_action("bash", None), PermissionAction::Ask);
    }

    #[test]
    fn effective_action_denies_sensitive_paths() {
        let rules = PermissionSet::default_rules();
        assert_eq!(rules.effective_action("read_file", Some("/home/user/.ssh/id_rsa")), PermissionAction::Deny);
        assert_eq!(rules.effective_action("write_file", Some("/project/.env")), PermissionAction::Deny);
        // Non-sensitive paths still follow the ruleset.
        assert_eq!(rules.effective_action("read_file", Some("/project/src/main.rs")), PermissionAction::Allow);
    }
}
