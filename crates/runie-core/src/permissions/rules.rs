//! Permission rulesets with wildcard matching.
//!
//! Evaluation: last-match wins. Sensitive paths are always denied.

use glob::Pattern;
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
        Pattern::new(&self.tool_pattern).is_ok_and(|p| p.matches(tool))
    }
    fn matches_path(&self, path: &str) -> bool {
        match &self.path_pattern {
            Some(p) => Pattern::new(p).is_ok_and(|pat| pat.matches(path)),
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
            PermissionRule {
                tool_pattern: "read_file".into(),
                path_pattern: None,
                action: PermissionAction::Allow,
            },
            PermissionRule {
                tool_pattern: "list_dir".into(),
                path_pattern: None,
                action: PermissionAction::Allow,
            },
            PermissionRule {
                tool_pattern: "grep".into(),
                path_pattern: None,
                action: PermissionAction::Allow,
            },
            PermissionRule {
                tool_pattern: "find".into(),
                path_pattern: None,
                action: PermissionAction::Allow,
            },
            PermissionRule {
                tool_pattern: "fetch_docs".into(),
                path_pattern: None,
                action: PermissionAction::Allow,
            },
            PermissionRule {
                tool_pattern: "write_file".into(),
                path_pattern: None,
                action: PermissionAction::Ask,
            },
            PermissionRule {
                tool_pattern: "edit_file".into(),
                path_pattern: None,
                action: PermissionAction::Ask,
            },
            PermissionRule {
                tool_pattern: "bash".into(),
                path_pattern: None,
                action: PermissionAction::Ask,
            },
        ])
    }

    pub fn rules(&self) -> &[PermissionRule] {
        &self.rules
    }
}
