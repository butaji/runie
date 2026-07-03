//! Grok → Runie tool name and schema alias map for deterministic comparisons.
//!
//! Grok Build and Runie use different tool naming conventions. This module
//! provides bidirectional mappings between tool names and schemas so that
//! fixture data can be compared across both systems.
//!
//! ## Tool Name Mapping
//!
//! | Grok Tool | Runie Tool |
//! |-----------|------------|
//! | `grok-read` | `Read` |
//! | `grok-write` | `Write` |
//! | `grok-list-directory` | `ListDir` |
//! | `grok-web-search` | `WebSearch` |
//! | `grok-web-fetch` | `WebFetch` |
//! | `grok-bash` | `Bash` |
//! | `grok-grep` | `Grep` |
//! | `grok-find` | `Find` |
//!
//! ## Schema Mapping
//!
//! Input/output schemas differ slightly between Grok and Runie. This module
//! provides conversion functions for common schema transformations.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Map a Grok tool name to the equivalent Runie tool name.
pub fn grok_to_runie(grok_tool: &str) -> Option<&'static str> {
    GROK_TO_RUNIE.get(grok_tool).copied()
}

/// Map a Runie tool name to the equivalent Grok tool name.
/// Returns the primary (first) Grok tool name that maps to the given Runie tool.
pub fn runie_to_grok(runie_tool: &str) -> Option<&'static str> {
    // Linear scan since HashMap reverses the mapping
    for (grok, runie) in GROK_TO_RUNIE.iter() {
        if *runie == runie_tool {
            return Some(grok);
        }
    }
    None
}

/// Get all Grok tool names that map to a Runie tool name.
pub fn runie_to_grok_all(runie_tool: &str) -> Vec<&'static str> {
    GROK_TO_RUNIE
        .iter()
        .filter(|(_, runie)| **runie == runie_tool)
        .map(|(grok, _)| *grok)
        .collect()
}

/// Static map: Grok tool name → Runie tool name.
static GROK_TO_RUNIE: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        // File operations
        m.insert("grok-read", "Read");
        m.insert("grok-write", "Write");
        m.insert("grok-edit", "Edit");
        m.insert("grok-create", "Create");
        m.insert("grok-delete", "Delete");
        m.insert("grok-move", "Move");
        m.insert("grok-copy", "Copy");
        // Directory operations
        m.insert("grok-list-directory", "ListDir");
        m.insert("grok-list-files", "ListDir");
        m.insert("grok-directory-tree", "Tree");
        // Search
        m.insert("grok-web-search", "WebSearch");
        m.insert("grok-web-fetch", "WebFetch");
        m.insert("grok-search", "Grep");
        m.insert("grok-grep", "Grep");
        m.insert("grok-find", "Find");
        m.insert("grok-glob", "Glob");
        // Shell
        m.insert("grok-bash", "Bash");
        m.insert("grok-shell", "Bash");
        m.insert("grok-exec", "Bash");
        m.insert("grok-run", "Bash");
        // System
        m.insert("grok-open", "Open");
        m.insert("grok-clipboard-read", "ClipboardRead");
        m.insert("grok-clipboard-write", "ClipboardWrite");
        // Code intelligence
        m.insert("grok-definitions", "Definitions");
        m.insert("grok-references", "References");
        m.insert("grok-document-symbols", "DocumentSymbols");
        m.insert("grok-completion", "Completion");
        // Inspectors
        m.insert("grok-inspect", "Inspect");
        m.insert("grok-explain", "Inspect");
        // Planning
        m.insert("grok-plan", "Plan");
        m.insert("grok-think", "Think");
        // Misc
        m.insert("grok-mcp", "Mcp");
        m.insert("grok-task", "Task");
        m
    });

/// Schema transformation for tool arguments.
///
/// Grok uses snake_case for arguments, Runie uses camelCase.
/// Some tools have different argument names.
pub fn transform_args(tool_name: &str, args: &serde_json::Value) -> serde_json::Value {
    // Most tools don't need transformation
    match tool_name {
        // Bash: grok uses `command`, Runie uses `cmd`
        "Bash" | "grok-bash" => {
            if let Some(obj) = args.as_object() {
                let mut new_obj = obj.clone();
                if let Some(cmd) = obj.get("command").cloned() {
                    new_obj.insert("cmd".to_string(), cmd);
                    new_obj.remove("command");
                }
                serde_json::json!(new_obj)
            } else {
                args.clone()
            }
        }
        // Read: grok uses `path`, Runie uses `path` (same)
        "Read" | "grok-read" => args.clone(),
        // Write: grok uses `path` and `content`, Runie uses `path` and `content` (same)
        "Write" | "grok-write" => args.clone(),
        // Grep: argument names differ slightly
        "Grep" | "grok-grep" => {
            if let Some(obj) = args.as_object() {
                let mut new_obj = serde_json::Map::new();
                // Map grok's `pattern` to runie's `pattern`
                if let Some(v) = obj.get("pattern").or(obj.get("regex")).or(obj.get("search")) {
                    new_obj.insert("pattern".to_string(), v.clone());
                }
                if let Some(v) = obj.get("path").or(obj.get("file")).or(obj.get("directory")) {
                    new_obj.insert("path".to_string(), v.clone());
                }
                if let Some(v) = obj.get("context") {
                    new_obj.insert("context".to_string(), v.clone());
                }
                if new_obj.is_empty() {
                    args.clone()
                } else {
                    serde_json::json!(new_obj)
                }
            } else {
                args.clone()
            }
        }
        // Default: no transformation
        _ => args.clone(),
    }
}

/// Check if a tool is read-only (safe to cache).
pub fn is_read_only_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Read"
            | "ListDir"
            | "Grep"
            | "Find"
            | "Glob"
            | "WebSearch"
            | "WebFetch"
            | "Tree"
            | "Definitions"
            | "References"
            | "DocumentSymbols"
            | "Inspect"
    )
}

/// Normalize a tool name for comparison.
///
/// Strips prefixes like `grok-` and normalizes case.
pub fn normalize_tool_name(name: &str) -> String {
    let name = name.trim();
    let name = name.strip_prefix("grok-").unwrap_or(name);
    // Title case for comparison
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Get the canonical tool name for display.
///
/// Maps both Grok and Runie names to a display form.
pub fn display_name(tool_name: &str) -> String {
    // First try Grok to Runie mapping
    if let Some(runie) = grok_to_runie(tool_name) {
        return runie.to_string();
    }
    // If it's already a Runie tool name, return it as-is
    // (runie_to_grok returns the grok alias, so we just return the input)
    if runie_to_grok(tool_name).is_some() {
        return tool_name.to_string();
    }
    // Fall back to normalized
    normalize_tool_name(tool_name)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grok_to_runie() {
        assert_eq!(grok_to_runie("grok-read"), Some("Read"));
        assert_eq!(grok_to_runie("grok-write"), Some("Write"));
        assert_eq!(grok_to_runie("grok-bash"), Some("Bash"));
        assert_eq!(grok_to_runie("grok-list-directory"), Some("ListDir"));
        assert_eq!(grok_to_runie("grok-grep"), Some("Grep"));
        assert_eq!(grok_to_runie("grok-web-search"), Some("WebSearch"));
        assert_eq!(grok_to_runie("unknown-tool"), None);
    }

    #[test]
    fn test_runie_to_grok() {
        assert_eq!(runie_to_grok("Read"), Some("grok-read"));
        assert_eq!(runie_to_grok("Write"), Some("grok-write"));
        // Bash could be grok-bash, grok-shell, grok-exec, or grok-run
        let bash_aliases = runie_to_grok_all("Bash");
        assert!(bash_aliases.contains(&"grok-bash") || bash_aliases.contains(&"grok-shell"),
            "Bash should map to grok-bash or grok-shell");
        // ListDir could be grok-list-directory or grok-list-files
        let listdir_aliases = runie_to_grok_all("ListDir");
        assert!(listdir_aliases.contains(&"grok-list-directory") || listdir_aliases.contains(&"grok-list-files"),
            "ListDir should map to grok-list-directory or grok-list-files");
        assert_eq!(runie_to_grok("unknown-tool"), None);
    }

    #[test]
    fn test_bash_args_transform() {
        let args = serde_json::json!({
            "command": "ls -la"
        });
        let transformed = transform_args("Bash", &args);
        assert!(transformed.get("cmd").is_some());
        assert!(transformed.get("command").is_none());
    }

    #[test]
    fn test_grep_args_transform() {
        let args = serde_json::json!({
            "regex": "fn main",
            "file": "src/main.rs"
        });
        let transformed = transform_args("Grep", &args);
        assert_eq!(transformed.get("pattern").and_then(|v| v.as_str()), Some("fn main"));
        assert_eq!(transformed.get("path").and_then(|v| v.as_str()), Some("src/main.rs"));
    }

    #[test]
    fn test_read_only_tools() {
        assert!(is_read_only_tool("Read"));
        assert!(is_read_only_tool("ListDir"));
        assert!(is_read_only_tool("Grep"));
        assert!(is_read_only_tool("Find"));
        assert!(is_read_only_tool("WebSearch"));
        assert!(is_read_only_tool("WebFetch"));
        assert!(!is_read_only_tool("Write"));
        assert!(!is_read_only_tool("Bash"));
        assert!(!is_read_only_tool("Edit"));
    }

    #[test]
    fn test_normalize_tool_name() {
        assert_eq!(normalize_tool_name("grok-read"), "Read");
        assert_eq!(normalize_tool_name("grok-write"), "Write");
        assert_eq!(normalize_tool_name("grok-bash"), "Bash");
        assert_eq!(normalize_tool_name("Read"), "Read");
        assert_eq!(normalize_tool_name("BASH"), "BASH");
    }

    #[test]
    fn test_display_name() {
        // Grok tool names should map to Runie names
        assert_eq!(display_name("grok-read"), "Read");
        assert_eq!(display_name("grok-write"), "Write");
        // Runie tool names should stay as-is
        assert_eq!(display_name("Read"), "Read");
        assert_eq!(display_name("Write"), "Write");
        // Unknown tools should be normalized (title case, grok- prefix stripped)
        assert_eq!(display_name("customtool"), "Customtool"); // Not a known tool
        assert_eq!(display_name("grok-unknown"), "Unknown"); // grok- prefix stripped
    }
}
