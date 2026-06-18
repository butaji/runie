//! Tool marker parsing utilities.
//!
//! Provides functions to detect and strip tool call markers from text content.
//! This module unifies the logic between runie-core and runie-agent to prevent
//! drift and handle edge cases properly.

mod strip;

use serde_json::Value;

const TOOL_CALL_START: &str = "[TOOL_CALL]";
const TOOL_CALL_END: &str = "[/TOOL_CALL]";

const KNOWN_TOOLS: &[&str] = &[
    "ask_user",
    "bash",
    "read_file",
    "write_file",
    "edit_file",
    "list_dir",
    "grep",
    "find",
    "fetch_docs",
    "search",
    "find_definitions",
    "select_model",
    "done",
    "list_subagents",
    "cancel_subagent",
    "get_subagent_status",
    "get_subagent_output",
    "steer_subagent",
];

/// Checks if the given text contains any tool call markers.
///
/// A tool call marker is either:
/// - A line starting with "TOOL:" (legacy format)
/// - A JSON object with both "name" and "arguments" fields (structured format)
/// - A `[TOOL_CALL]{tool => "...", args => {...}}[/TOOL_CALL]` block (markup format)
///
/// Note: This function is conservative - it may return true for text that
/// looks like a tool call but isn't. Use `parse_tool_calls` for precise parsing.
pub fn has_tool_markers(text: &str) -> bool {
    !parse_tool_calls(text).is_empty()
}

/// Parses tool calls from text content.
///
/// Returns a list of tool names found in the text.
/// A tool call is identified by:
/// - Legacy: Line starting with "TOOL:" followed by tool name
/// - Structured: JSON with "name" and "arguments" fields
/// - Markup: `[TOOL_CALL]{tool => "name", args => {...}}[/TOOL_CALL]`
/// - MiniMax XML: `<minimax:tool_call><invoke name="..."></invoke></minimax:tool_call>`
pub fn parse_tool_calls(text: &str) -> Vec<String> {
    let mut tools = parse_minimax_tool_call_names(text);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check for legacy TOOL: format
        if let Some(rest) = line.strip_prefix("TOOL:") {
            let name = rest
                .split(|c: char| c == ':' || c.is_whitespace())
                .next()
                .unwrap_or("");
            if !name.is_empty() {
                tools.push(name.to_string());
            }
            continue;
        }

        // Check for structured JSON format
        if line.starts_with('{') {
            if let Some(name) = parse_json_tool_name(line) {
                tools.push(name);
            }
            continue;
        }

        // Check for [TOOL_CALL] markup format
        if let Some(name) = parse_tool_call_markup_name(line) {
            tools.push(name);
        }
    }
    tools
}

/// Strips tool call markers from text content.
///
/// Returns the text with all tool call markers removed.
/// This preserves legitimate text that happens to contain "TOOL:" as a word.
pub fn strip_tool_markers(content: &str) -> String {
    strip::strip_all(content)
}

fn parse_minimax_tool_call_names(text: &str) -> Vec<String> {
    const OPEN: &str = "<minimax:tool_call>";
    const CLOSE: &str = "</minimax:tool_call>";
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            break;
        };
        let block = &after_open[..end];
        names.extend(extract_minimax_invoke_names(block));
        rest = &after_open[end + CLOSE.len()..];
    }
    names
}

fn extract_minimax_invoke_names(block: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = block;
    while let Some(start) = rest.find("<invoke") {
        let after_tag = &rest[start + "<invoke".len()..];
        let Some(close) = after_tag.find('>') else {
            break;
        };
        if let Some(name) = extract_xml_attr(&after_tag[..close], "name") {
            names.push(name);
        }
        rest = &after_tag[close + 1..];
    }
    names
}

fn extract_xml_attr(tag: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=", key);
    let mut rest = tag;
    while let Some(idx) = rest.find(&pattern) {
        let after_key = &rest[idx + pattern.len()..];
        let quote = after_key.chars().next()?;
        if quote != '\'' && quote != '"' {
            rest = &after_key[1..];
            continue;
        }
        let after_quote = &after_key[1..];
        let end = after_quote.find(quote)?;
        return Some(after_quote[..end].to_string());
    }
    None
}

fn parse_json_tool_name(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("name").is_some() && value.get("arguments").is_some() {
        let name = value.get("name").and_then(|v| v.as_str())?;
        if is_known_tool(name) {
            Some(name.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

fn is_known_tool(name: &str) -> bool {
    KNOWN_TOOLS.contains(&name)
}

pub(crate) fn is_tool_call_value(value: &Value) -> bool {
    value.get("name").is_some()
        && value.get("arguments").is_some()
        && value
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(is_known_tool)
}

fn parse_tool_call_markup_name(line: &str) -> Option<String> {
    let payload = extract_tool_call_payload(line)?;
    let json = arrow_to_json(payload);
    let value: Value = serde_json::from_str(&json).ok()?;
    value.get("tool").and_then(|v| v.as_str()).map(String::from)
}

fn extract_tool_call_payload(line: &str) -> Option<&str> {
    let start = line.find(TOOL_CALL_START)?;
    let after_start = &line[start + TOOL_CALL_START.len()..];
    let end = after_start.find(TOOL_CALL_END)?;
    Some(after_start[..end].trim())
}

fn arrow_to_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            in_string = !in_string;
            out.push('"');
            continue;
        }
        if !in_string && ch == '=' && chars.peek() == Some(&'>') {
            chars.next();
            out.push(':');
            if chars.peek() == Some(&' ') {
                chars.next();
                out.push(' ');
            }
            continue;
        }
        if !in_string && (ch.is_alphabetic() || ch == '_') {
            let mut word = String::new();
            word.push(ch);
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let last = out.trim_end().chars().last();
            let is_key = last == Some('{') || last == Some(',');
            if is_key {
                out.push('"');
                out.push_str(&word);
                out.push('"');
            } else {
                out.push_str(&word);
            }
            continue;
        }
        out.push(ch);
    }
    out
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
    fn test_parse_tool_calls_invalid_json() {
        let input = "{invalid json}";
        let tools = parse_tool_calls(input);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_markup_format() {
        let input = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "echo hi"}}[/TOOL_CALL]"#;
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash"]);
    }

    #[test]
    fn test_parse_tool_calls_markup_unknown_tool() {
        let input = r#"[TOOL_CALL]{tool => "unknown_tool", args => {}}[/TOOL_CALL]"#;
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["unknown_tool"]);
    }

    #[test]
    fn test_parse_tool_calls_malformed_markup() {
        let input = r#"[TOOL_CALL]{tool => "bash", args => {}}"#;
        let tools = parse_tool_calls(input);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_has_tool_markers_minimax() {
        let input = r#"<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>"#;
        assert!(has_tool_markers(input));
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
    fn test_parse_tool_calls_minimax_multiple_invokes() {
        let input = r#"<minimax:tool_call>
<invoke name="read_file">
<parameter name="path">a.txt</parameter>
</invoke>
<invoke name="bash">
<parameter name="command">echo hi</parameter>
</invoke>
</minimax:tool_call>"#;
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["read_file", "bash"]);
    }

    #[test]
    fn test_strip_tool_markers_inline_json() {
        let input = r#"Here's the result: {"name": "read_file", "arguments": {"path": "/test"}} Done."#;
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the result:  Done.");
    }

    #[test]
    fn test_strip_tool_markers_code_fenced_json() {
        let input = "→ ```json\n{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}\n```\nHere's the current directory.";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_tool_markers_fenced_inline_json() {
        let input = "→ ```json{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}Here's the current directory.";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_tool_markers_preserves_legitimate_json() {
        let input = r#"Example config: {"name": "foo", "arguments": {"x": 1}}."#;
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_tool_markers_preserves_code_block() {
        let input = "```json\n{\"name\": \"foo\"}\n```";
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
    }
}
