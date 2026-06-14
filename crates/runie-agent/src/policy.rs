//! Tool policy groups and glob-based path matching.
//!
//! Tools are classified into policy groups (read-only, write, dangerous).
//! Path-based rules use glob patterns to allow or deny tool execution.

use std::path::Path;

/// Policy group for a tool. Determines the default approval requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPolicyGroup {
    /// Read-only tools that never modify the filesystem.
    ReadOnly,
    /// Tools that write to the filesystem but in controlled ways.
    Write,
    /// Tools that execute arbitrary code or shell commands.
    Dangerous,
}

impl ToolPolicyGroup {
    /// Returns true if tools in this group require user approval by default.
    pub fn requires_approval(&self) -> bool {
        match self {
            ToolPolicyGroup::ReadOnly => false,
            ToolPolicyGroup::Write => true,
            ToolPolicyGroup::Dangerous => true,
        }
    }
}

/// A single path rule using a glob pattern.
#[derive(Debug, Clone)]
pub struct PathRule {
    /// Glob pattern (e.g., `"**/*.rs"`, `"~/.ssh/*"`).
    pub pattern: String,
    /// If true, matching this pattern is allowed; if false, denied.
    pub allow: bool,
}

impl PathRule {
    /// Create a new allow rule for the given glob pattern.
    pub fn allow(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into(), allow: true }
    }

    /// Create a new deny rule for the given glob pattern.
    pub fn deny(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into(), allow: false }
    }

    /// Check if this rule matches the given path.
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path_to_glob_string(path);
        glob_match(&self.pattern, &path_str)
    }
}

/// Represents the combined policy for a tool: group + optional path rules.
#[derive(Debug, Clone)]
pub struct ToolPolicy {
    /// The policy group this tool belongs to.
    pub group: ToolPolicyGroup,
    /// Optional path rules. First matching rule (by order) wins.
    pub path_rules: Vec<PathRule>,
}

impl ToolPolicy {
    /// Get the effective approval requirement after applying path rules.
    /// Returns `Some(false)` if path is denied, `None` if no matching rule,
    /// or `Some(true/false)` based on the group default.
    pub fn effective_approval(&self, path: Option<&Path>) -> Option<bool> {
        if let Some(path) = path {
            for rule in &self.path_rules {
                if rule.matches(path) {
                    return Some(rule.allow);
                }
            }
        }
        None
    }

    /// Default policy groups for built-in tool names.
    pub fn for_tool_name(name: &str) -> Option<ToolPolicyGroup> {
        match name {
            "read_file" | "list_dir" | "grep" | "find" | "fetch_docs" => {
                Some(ToolPolicyGroup::ReadOnly)
            }
            "write_file" | "edit_file" => Some(ToolPolicyGroup::Write),
            "bash" => Some(ToolPolicyGroup::Dangerous),
            _ => None,
        }
    }
}

/// Returns true if `pattern` matches `text` using glob rules.
/// Supports `*` (any characters except `/`), `**` (any characters including `/`),
/// and `?` (single character except `/`).
pub fn glob_match(pattern: &str, text: &str) -> bool {
    // Resolve ~ at the start of patterns to the home directory
    let pattern = if pattern.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut p = std::path::PathBuf::from(home);
            p.push(&pattern[2..]);
            p.to_string_lossy().into_owned()
        } else {
            pattern.to_string()
        }
    } else {
        pattern.to_string()
    };

    glob::Pattern::new(&pattern)
        .map(|p| p.matches(text))
        .unwrap_or(false)
}

pub(super) fn path_to_glob_string(path: &Path) -> String {
    // Normalize paths to use forward slashes for glob compatibility
    let s = path.to_string_lossy();
    if s.contains('\\') { s.replace('\\', "/").to_string() } else { s.into_owned() }
}
