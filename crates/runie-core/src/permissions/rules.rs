//! Permission rulesets with wildcard matching.
//!
//! Evaluation: last-match wins. Sensitive paths are always denied.

use crate::glob::matches;
use serde::{Deserialize, Serialize};

use super::PermissionAction;

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
        matches(&self.tool_pattern, tool)
    }
    fn matches_path(&self, path: &str) -> bool {
        match &self.path_pattern {
            Some(p) => matches(p, path),
            None => true,
        }
    }
    pub fn matches(&self, tool: &str, path: Option<&str>) -> bool {
        if !self.matches_tool(tool) {
            return false;
        }
        match path {
            Some(p) => self.matches_path(p),
            None => self.path_pattern.is_none(),
        }
    }
}

/// A set of permission rules (last-match wins).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    rules: Vec<PermissionRule>,
}

impl PermissionSet {
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self { rules }
    }

    /// Evaluate tool+path against rules (last-match wins).
    pub fn evaluate(&self, tool: &str, path: Option<&str>) -> PermissionAction {
        let mut result = PermissionAction::Ask;
        for rule in &self.rules {
            if rule.matches(tool, path) {
                result = rule.action;
            }
        }
        result
    }

    /// Evaluate with the built-in sensitive-path denylist applied first.
    pub fn effective_action(&self, tool: &str, path: Option<&str>) -> PermissionAction {
        if let Some(p) = path {
            if super::is_sensitive_path(p) {
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
            allow_rule("read_file"),
            allow_rule("list_dir"),
            allow_rule("grep"),
            allow_rule("find"),
            allow_rule("fetch_docs"),
            ask_rule("write_file"),
            ask_rule("edit_file"),
            ask_rule("bash"),
        ])
    }

    pub fn rules(&self) -> &[PermissionRule] {
        &self.rules
    }
}

fn allow_rule(tool: &str) -> PermissionRule {
    PermissionRule {
        tool_pattern: tool.into(),
        path_pattern: None,
        action: PermissionAction::Allow,
    }
}

fn ask_rule(tool: &str) -> PermissionRule {
    PermissionRule {
        tool_pattern: tool.into(),
        path_pattern: None,
        action: PermissionAction::Ask,
    }
}
