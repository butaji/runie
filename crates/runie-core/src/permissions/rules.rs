//! Permission rulesets with wildcard matching and declarative configuration.
//!
//! Evaluation: last-match wins. Sensitive paths are always denied.
//!
//! ## Rule Format
//!
//! ```toml
//! [[permissions]]
//! action = "allow"
//! tool = "read_file"
//!
//! [[permissions]]
//! action = "deny"
//! tool = "bash"
//! pattern = "rm -rf /"
//!
//! [[permissions]]
//! action = "ask"
//! tool = "write_file"
//! pattern = "*.rs"
//! scope = "project"
//! ```

use glob::Pattern;
use serde::{Deserialize, Serialize};

use super::PermissionAction;

/// Scope of a permission rule.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    /// User-level rules from ~/.runie/config.toml
    #[default]
    User,
    /// Project-level rules from .runie/config.toml or AGENTS.md
    Project,
    /// Session-level rules from CLI flags
    Session,
}

/// A single permission rule with glob patterns and scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PermissionRule {
    /// Action when rule matches: "allow", "ask", or "deny".
    pub action: PermissionAction,
    /// Glob pattern for tool name, e.g. "bash", "read_*", "*".
    pub tool: String,
    /// Optional glob pattern for file paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional glob pattern for shell command arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Scope of this rule: user, project, or session.
    /// Higher scopes override lower scopes (session > project > user).
    #[serde(default)]
    pub scope: PermissionScope,
}

impl PermissionRule {
    /// Create a rule from an action and tool pattern.
    pub fn new(action: PermissionAction, tool: impl Into<String>) -> Self {
        Self {
            action,
            tool: tool.into(),
            path: None,
            pattern: None,
            scope: PermissionScope::User,
        }
    }

    /// Set the path pattern for this rule.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the command pattern for this rule.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Set the scope for this rule.
    pub fn with_scope(mut self, scope: PermissionScope) -> Self {
        self.scope = scope;
        self
    }

    fn matches_tool(&self, tool: &str) -> bool {
        glob_matches(&self.tool, tool)
    }

    fn matches_path(&self, path: &str) -> bool {
        match &self.path {
            Some(p) => glob_matches(p, path),
            None => true,
        }
    }

    fn matches_pattern(&self, cmd: &str) -> bool {
        match &self.pattern {
            Some(p) => glob_matches(p, cmd),
            None => true,
        }
    }

    /// Check if this rule matches the given tool, path, and command.
    pub fn matches(&self, tool: &str, path: Option<&str>, cmd: Option<&str>) -> bool {
        if !self.matches_tool(tool) {
            return false;
        }
        match path {
            Some(p) if !self.matches_path(p) => return false,
            Some(_) => {}
            None if self.path.is_some() => return false,
            None => {}
        }
        match cmd {
            Some(c) if !self.matches_pattern(c) => return false,
            Some(_) => {}
            None if self.pattern.is_some() => return false,
            None => {}
        }
        true
    }
}

/// A set of permission rules with layered evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    rules: Vec<PermissionRule>,
}

impl PermissionSet {
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self { rules }
    }

    /// Add a rule to the set.
    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    /// Extend with another set of rules.
    pub fn extend(&mut self, other: PermissionSet) {
        self.rules.extend(other.rules);
    }

    /// Evaluate tool+path against rules (last-match wins within each scope).
    pub fn evaluate(&self, tool: &str, path: Option<&str>, cmd: Option<&str>) -> PermissionAction {
        let mut result = PermissionAction::Ask;
        for rule in &self.rules {
            if rule.matches(tool, path, cmd) {
                result = rule.action;
            }
        }
        result
    }

    /// Evaluate with the built-in sensitive-path denylist applied first.
    pub fn effective_action(
        &self,
        tool: &str,
        path: Option<&str>,
        cmd: Option<&str>,
    ) -> PermissionAction {
        if let Some(p) = path {
            if super::is_sensitive_path(p) {
                return PermissionAction::Deny;
            }
        }
        self.evaluate(tool, path, cmd)
    }

    /// Evaluate with scope precedence: session > project > user.
    /// Returns the highest-priority matching rule's action.
    /// Higher scopes always override lower scopes, even if they return Ask.
    pub fn evaluate_with_scope(
        &self,
        tool: &str,
        path: Option<&str>,
        cmd: Option<&str>,
        max_scope: PermissionScope,
    ) -> PermissionAction {
        // Track whether each scope has a matching rule
        let mut scope_has_match = [false, false, false];
        let mut scope_actions = [
            PermissionAction::Ask,
            PermissionAction::Ask,
            PermissionAction::Ask,
        ];

        for rule in &self.rules {
            // Skip rules with higher scope than max_scope
            if scope_priority(&rule.scope) > scope_priority(&max_scope) {
                continue;
            }
            if rule.matches(tool, path, cmd) {
                let idx = scope_index(&rule.scope);
                scope_has_match[idx] = true;
                scope_actions[idx] = rule.action;
            }
        }

        // Apply sensitive path denylist
        if let Some(p) = path {
            if super::is_sensitive_path(p) {
                return PermissionAction::Deny;
            }
        }

        // Return the highest-priority scope's action if it has a matching rule
        // Session (2) > Project (1) > User (0)
        for idx in [2, 1, 0] {
            if scope_has_match[idx] {
                return scope_actions[idx];
            }
        }
        PermissionAction::Ask
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

    /// Rules for acceptEdits mode: auto-approve file edits.
    pub fn accept_edits_rules() -> Self {
        Self::new(vec![
            allow_rule("read_file"),
            allow_rule("list_dir"),
            allow_rule("grep"),
            allow_rule("find"),
            allow_rule("fetch_docs"),
            allow_rule("write_file"),
            allow_rule("edit_file"),
            ask_rule("bash"),
        ])
    }

    /// Rules for dontAsk mode: deny only on explicit deny rules.
    pub fn dont_ask_rules() -> Self {
        Self::new(vec![
            allow_rule("read_file"),
            allow_rule("list_dir"),
            allow_rule("grep"),
            allow_rule("find"),
            allow_rule("fetch_docs"),
            allow_rule("write_file"),
            allow_rule("edit_file"),
            allow_rule("bash"),
        ])
    }

    /// Get all rules.
    pub fn rules(&self) -> &[PermissionRule] {
        &self.rules
    }

    /// Get rules for a specific scope.
    pub fn rules_for_scope(&self, scope: PermissionScope) -> Vec<&PermissionRule> {
        self.rules.iter().filter(|r| r.scope == scope).collect()
    }
}

fn scope_priority(scope: &PermissionScope) -> usize {
    match scope {
        PermissionScope::User => 0,
        PermissionScope::Project => 1,
        PermissionScope::Session => 2,
    }
}

fn scope_index(scope: &PermissionScope) -> usize {
    match scope {
        PermissionScope::User => 0,
        PermissionScope::Project => 1,
        PermissionScope::Session => 2,
    }
}

fn allow_rule(tool: &str) -> PermissionRule {
    PermissionRule::new(PermissionAction::Allow, tool)
}

fn ask_rule(tool: &str) -> PermissionRule {
    PermissionRule::new(PermissionAction::Ask, tool)
}

/// Match a glob pattern against a string using the `glob` crate.
fn glob_matches(pattern: &str, name: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}
