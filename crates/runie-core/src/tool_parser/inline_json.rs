//! Inline JSON tool-call parsing.

use serde_json::Value;

use crate::tool_parser::minimax::is_known_tool;

use super::{ParsedToolCall, ToolParseError};

/// Parse inline JSON tool calls from a line.
///
/// Scans the line for JSON objects that match the tool call schema.
pub fn parse_inline_json_tools(line: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        if let Some((end, value)) = parse_json_object_at(bytes, i) {
            if let Some(call) = value_to_tool_call(&value) {
                results.push(Ok(call));
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    results
}

fn parse_json_object_at(bytes: &[u8], start: usize) -> Option<(usize, Value)> {
    let end = find_object_end(bytes, start)?;
    let slice = std::str::from_utf8(&bytes[start..=end]).ok()?;
    let value: Value = serde_json::from_str(slice).ok()?;
    Some((end, value))
}

fn find_object_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn value_to_tool_call(value: &Value) -> Option<ParsedToolCall> {
    let name = value.get("name").and_then(|v| v.as_str())?;
    let args = value.get("arguments").and_then(|v| v.as_object())?;
    if !is_known_tool(name) {
        return None;
    }
    Some(ParsedToolCall {
        name: name.to_string(),
        args: Value::Object(args.clone()),
        id: None,
    })
}

/// Check if a line has inline JSON tool calls.
pub fn has_inline_json_tools(line: &str) -> bool {
    if !line.contains('{') {
        return false;
    }
    let results = parse_inline_json_tools(line);
    !results.is_empty()
}
