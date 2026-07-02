//! MCP tool annotation registry for built-in tools.
//!
//! Maps built-in tool names to their MCP `ToolAnnotations`, enabling the
//! permission gate to read `read_only_hint` and `open_world_hint` from a
//! single canonical source instead of hardcoded strings.
//!
//! This replaces the legacy `is_read_only_tool` function with a proper
//! MCP annotation lookup.

use rmcp::model::ToolAnnotations;

/// Look up MCP `ToolAnnotations` for a built-in tool by name.
///
/// Returns `Some(annotations)` for known built-in tools, `None` for unknown
/// tools (e.g. MCP server tools).
///
/// # Annotation semantics
///
/// - `read_only_hint = Some(true)`: tool has no side effects; safe for auto-approval.
/// - `read_only_hint = Some(false)`: tool may modify files/processes.
/// - `open_world_hint = Some(true)`: tool makes network requests.
pub fn get_tool_annotations(tool: &str) -> Option<ToolAnnotations> {
    match tool {
        // ── Read-only tools ────────────────────────────────────────────────────
        "read_file" => Some(ToolAnnotations::new().read_only(true)),
        "grep" => Some(ToolAnnotations::new().read_only(true)),
        "find" => Some(ToolAnnotations::new().read_only(true)),
        "list_dir" => Some(ToolAnnotations::new().read_only(true)),
        "search" => Some(ToolAnnotations::new().read_only(true)),
        "find_definitions" => Some(ToolAnnotations::new().read_only(true)),

        // ── Read-only with network access ──────────────────────────────────────
        "fetch_docs" => Some(ToolAnnotations::new().read_only(true).open_world(true)),

        // ── File-modifying tools ────────────────────────────────────────────────
        "write_file" => Some(ToolAnnotations::new().read_only(false)),
        "edit_file" => Some(ToolAnnotations::new().read_only(false)),
        "bash" => Some(ToolAnnotations::new().read_only(false)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_tools_have_read_only_hint() {
        for tool in ["read_file", "grep", "find", "list_dir", "search", "find_definitions"] {
            let ann = get_tool_annotations(tool).expect(tool);
            assert_eq!(ann.read_only_hint, Some(true), "{tool} should be read-only");
        }
    }

    #[test]
    fn modifying_tools_do_not_have_read_only_hint() {
        for tool in ["write_file", "edit_file", "bash"] {
            let ann = get_tool_annotations(tool).expect(tool);
            assert_eq!(ann.read_only_hint, Some(false), "{tool} should not be read-only");
        }
    }

    #[test]
    fn fetch_docs_has_open_world_hint() {
        let ann = get_tool_annotations("fetch_docs").unwrap();
        assert_eq!(ann.read_only_hint, Some(true));
        assert_eq!(ann.open_world_hint, Some(true));
    }

    #[test]
    fn unknown_tool_returns_none() {
        assert!(get_tool_annotations("unknown_tool").is_none());
    }
}
