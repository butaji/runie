//! [TOOL_CALL] markup parsing.

use serde_json::Value;

use crate::tool_parser::minimax::is_known_tool;

use super::ParsedToolCall;

const TOOL_CALL_START: &str = "[TOOL_CALL]";
const TOOL_CALL_END: &str = "[/TOOL_CALL]";

/// Parse [TOOL_CALL] markup from a line.
pub fn parse_tool_call_markup(line: &str) -> Option<ParsedToolCall> {
    let payload = extract_tool_call_payload(line)?;
    let json = arrow_to_json(payload);
    let value: Value = serde_json::from_str(&json).ok()?;
    let name = value.get("tool").and_then(|v| v.as_str())?;
    let args = value.get("args").and_then(|v| v.as_object())?;
    if !is_known_tool(name) {
        return None;
    }
    Some(ParsedToolCall {
        name: name.to_string(),
        args: Value::Object(args.clone()),
        id: None,
    })
}

fn extract_tool_call_payload(line: &str) -> Option<&str> {
    let start = line.find(TOOL_CALL_START)?;
    let after_start = &line[start + TOOL_CALL_START.len()..];
    let end = after_start.find(TOOL_CALL_END)?;
    Some(after_start[..end].trim())
}

/// Check if a line has [TOOL_CALL] markup.
pub fn has_tool_call_markup(line: &str) -> bool {
    line.contains("[TOOL_CALL]")
}

/// Convert arrow syntax (key => value) to JSON (key: value).
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
