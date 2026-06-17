//! Tool call parser — extracts tool invocations from LLM text output.
//!
//! Returns `(name, arguments)` tuples for use with `ToolRegistry`.

use serde_json::{Map, Value};

/// A parsed tool invocation: name and JSON arguments.
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: Value,
}

/// Parse tool calls from LLM text output.
/// Returns a list of `(tool_name, arguments)` tuples.
pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    let mut tools = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tool = if line.starts_with("TOOL:") {
            parse_legacy_tool(line.strip_prefix("TOOL:").unwrap_or(""))
        } else if line.starts_with('{') {
            parse_structured_tool(line)
        } else {
            None
        };
        if let Some(t) = tool {
            tools.push(t);
        }
    }
    tools
}

/// Check if text contains tool call markers.
pub fn has_tool_calls(text: &str) -> bool {
    runie_core::tool_markers::has_tool_markers(text)
}

// ─── Parsers ────────────────────────────────────────────────────────────────

fn parse_legacy_tool(payload: &str) -> Option<ParsedToolCall> {
    let parts: Vec<&str> = payload.splitn(3, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    let tool_name = parts[0];
    let arg1 = parts.get(1).unwrap_or(&"");
    let arg2 = parts.get(2).unwrap_or(&"");

    let mut args = Map::new();
    match tool_name {
        "read_file" => {
            args.insert("path".to_string(), Value::String(arg1.to_string()));
        }
        "list_dir" => {
            args.insert("path".to_string(), Value::String(arg1.to_string()));
        }
        "write_file" => {
            args.insert("path".to_string(), Value::String(arg1.to_string()));
            args.insert("content".to_string(), Value::String(arg2.to_string()));
        }
        "bash" => {
            args.insert("command".to_string(), Value::String(arg1.to_string()));
        }
        _ => return None,
    }
    Some(ParsedToolCall {
        name: tool_name.to_string(),
        args: Value::Object(args),
    })
}

fn parse_structured_tool(line: &str) -> Option<ParsedToolCall> {
    #[derive(Debug, serde::Deserialize)]
    struct ToolCall {
        name: String,
        arguments: Map<String, Value>,
    }
    let call: ToolCall = serde_json::from_str(line).ok()?;

    // Only parse known tool names
    let known_tools = [
        "read_file",
        "list_dir",
        "write_file",
        "edit_file",
        "bash",
        "grep",
        "find",
        "fetch_docs",
    ];
    if !known_tools.contains(&call.name.as_str()) {
        return None;
    }

    Some(ParsedToolCall {
        name: call.name,
        args: Value::Object(call.arguments),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_legacy_bash_tool() {
        let result = parse_tool_calls("TOOL:bash:ls -la");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "bash");
        assert_eq!(result[0].args["command"], "ls -la");
    }

    #[test]
    fn parse_legacy_read_file() {
        let result = parse_tool_calls("TOOL:read_file:src/main.rs");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "read_file");
        assert_eq!(result[0].args["path"], "src/main.rs");
    }

    #[test]
    fn parse_json_tool_call() {
        let json = r#"{"name": "bash", "arguments": {"command": "echo hi"}}"#;
        let result = parse_tool_calls(json);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "bash");
        assert_eq!(result[0].args["command"], "echo hi");
    }

    #[test]
    fn parse_multiple_tool_calls() {
        let text = r#"
TOOL:bash:ls
{"name": "read_file", "arguments": {"path": "Cargo.toml"}}
"#;
        let result = parse_tool_calls(text);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "bash");
        assert_eq!(result[1].name, "read_file");
    }
}
