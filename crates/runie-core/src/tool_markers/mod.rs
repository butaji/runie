//! Tool marker parsing utilities.
//!
//! This module is a thin facade over [`crate::tool::parse`] for the specific
//! needs of marker detection, name extraction, and stripping. The full parser
//! (with structured errors and argument extraction) lives in `tool::parse`.

mod strip;

/// Tool call markup delimiters.
pub const TOOL_CALL_START: &str = "[TOOL_CALL]";
pub const TOOL_CALL_END: &str = "[/TOOL_CALL]";

use crate::tool::parse::{has_tool_calls, is_tool_call_value, parse_tool_calls_fallible};

/// Strip tool-call artifacts from text content.
pub fn strip_tool_markers(content: &str) -> String {
    strip::strip_all(content)
}

/// Checks if the given text contains any tool call markers.
pub fn has_tool_markers(text: &str) -> bool {
    has_tool_calls(text)
}

/// Parses tool calls from text content and returns the tool names.
pub fn parse_tool_calls(text: &str) -> Vec<String> {
    parse_tool_calls_fallible(text)
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|call| call.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_tool_markers_positive() {
        assert!(has_tool_markers("TOOL:read_file /path/to/file"));
        assert!(has_tool_markers(
            r#"{"name": "read_file", "arguments": {"path": "/test"}}"#
        ));
        assert!(has_tool_markers(
            r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#
        ));
        assert!(has_tool_markers("Some text\nTOOL:bash ls\nAnother tool"));
    }

    #[test]
    fn test_has_tool_markers_negative() {
        assert!(!has_tool_markers("Hello, this is regular text."));
        assert!(!has_tool_markers(r#"{"foo": "bar"}"#));
        assert!(!has_tool_markers(r#"{"name": "not_a_tool"}"#));
        assert!(!has_tool_markers(r#"{"arguments": {}}"#));
        assert!(!has_tool_markers(
            r#"[TOOL_CALL]{tool => "bash", args => {}}"#
        ));
    }

    #[test]
    fn test_parse_tool_calls() {
        let input = "TOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}}";
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash", "read_file"]);
    }

    #[test]
    fn test_parse_tool_calls_markup_format() {
        let input = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "echo hi"}}[/TOOL_CALL]"#;
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash"]);
    }

    #[test]
    fn test_parse_tool_calls_minimax() {
        let input = "<minimax:tool_call>\n<invoke name=\"bash\">\n<parameter name=\"command\">ls</parameter>\n</invoke>\n</minimax:tool_call>";
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash"]);
    }

    #[test]
    fn test_strip_tool_markers_minimax() {
        let input = "I'll list files.\n<minimax:tool_call>\n<invoke name=\"list_dir\">\n<parameter name=\"path\">.</parameter>\n</invoke>\n</minimax:tool_call>\nDone.";
        let result = strip_tool_markers(input);
        assert_eq!(result, "I'll list files.\nDone.");
    }

    #[test]
    fn test_strip_tool_markers_inline_json() {
        let input = "Here's the result: {\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}} Done.";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the result:  Done.");
    }

    #[test]
    fn test_strip_tool_markers_preserves_legitimate_json() {
        let input = "Example config: {\"Name\": \"foo\", \"arguments\": {\"x\": 1}}.";
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_has_tool_markers_inline_legacy() {
        assert!(has_tool_markers("I'll list files.TOOL:list_dir:."));
        assert!(has_tool_markers(" preamble TOOL:bash:echo hi tail"));
    }

    #[test]
    fn test_strip_tool_markers_inline_legacy() {
        let input = "I'll list files.TOOL:list_dir:.";
        let result = strip_tool_markers(input);
        assert_eq!(result, "I'll list files.");
    }

    #[test]
    fn test_strip_tool_markers_preserves_inline_tool_mention() {
        let input = "Use the TOOL: parameter to configure the tool.";
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
    }
}
