//! Tool marker parsing utilities.
//!
//! This module is a thin facade over [`crate::tool_parser`] for the specific
//! needs of marker detection, name extraction, and stripping. The full parser
//! (with structured errors and argument extraction) lives in `tool_parser`.

mod strip;

use serde_json::Value;

const TOOL_CALL_START: &str = "[TOOL_CALL]";
const TOOL_CALL_END: &str = "[/TOOL_CALL]";

/// Strip tool-call artifacts from text content.
pub fn strip_tool_markers(content: &str) -> String {
    strip::strip_all(content)
}

/// Checks if the given text contains any tool call markers.
///
/// A tool call marker is either:
/// - A line starting with "TOOL:" (legacy format)
/// - A JSON object with both "name" and "arguments" fields (structured format)
/// - A `[TOOL_CALL]{tool => "...", args => {...}}[/TOOL_CALL]` block (markup format)
/// - A MiniMax XML `<minimax:tool_call>` block
///
/// Note: This function is conservative - it may return true for text that
/// looks like a tool call but isn't. Use `tool_parser::parse_tool_calls` for
/// precise parsing.
pub fn has_tool_markers(text: &str) -> bool {
    crate::tool_parser::has_tool_calls(text)
}

/// Parses tool calls from text content and returns the tool names.
///
/// This is a convenience wrapper around `tool_parser::parse_tool_calls` for
/// callers that only need the tool names.
pub fn parse_tool_calls(text: &str) -> Vec<String> {
    crate::tool_parser::parse_tool_calls(text)
        .into_iter()
        .map(|call| call.name)
        .collect()
}

pub(crate) fn is_tool_call_value(value: &Value) -> bool {
    crate::tool_parser::is_tool_call_value(value)
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
        let input = r#"<minimax:tool_call>
<invoke name="bash">
<parameter name="command">ls</parameter>
</invoke>
</minimax:tool_call>"#;
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash"]);
    }

    #[test]
    fn test_strip_tool_markers_minimax() {
        let input = r#"I'll list files.
<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>
Done."#;
        let result = strip_tool_markers(input);
        assert_eq!(result, "I'll list files.\nDone.");
    }

    #[test]
    fn test_strip_tool_markers_inline_json() {
        let input = r#"Here's the result: {"name": "read_file", "arguments": {"path": "/test"}} Done."#;
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the result:  Done.");
    }

    #[test]
    fn test_strip_tool_markers_preserves_legitimate_json() {
        let input = r#"Example config: {"name": "foo", "arguments": {"x": 1}}."#;
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
    }
}
