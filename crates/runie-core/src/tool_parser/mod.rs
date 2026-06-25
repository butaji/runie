//! Single source of truth for parsing tool-call markers from LLM text output.

pub mod minimax;

use crate::message::{ChatMessage, Part, ToolCall};
use serde_json::{Map, Value};
use minimax::is_known_tool;

const TOOL_CALL_START: &str = "[TOOL_CALL]";
const TOOL_CALL_END: &str = "[/TOOL_CALL]";

/// A parsed tool invocation: name, JSON arguments, and an optional call id.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: Value,
    pub id: Option<String>,
}

/// A tool-call parse error: the raw line and a human-readable reason.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolParseError {
    pub raw: String,
    pub reason: String,
}

/// Parse tool calls from LLM text output.
pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    parse_tool_calls_fallible(text)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

/// Parse tool calls, returning both successes and errors.
pub fn parse_tool_calls_fallible(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let minimax = parse_minimax_tool_calls(text);
    if !minimax.is_empty() {
        return minimax;
    }

    let mut results = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("TOOL:") {
            match parse_legacy_tool(rest) {
                Some(t) => results.push(Ok(t)),
                None if !rest.trim().is_empty() => results.push(Err(ToolParseError {
                    raw: line.to_string(),
                    reason: "invalid legacy TOOL syntax or unknown tool name".into(),
                })),
                None => {}
            }
            continue;
        }
        if line.contains('{') {
            let inline = parse_inline_json_tools(line);
            if !inline.is_empty() {
                results.extend(inline);
                continue;
            }
            if line.starts_with('{') {
                results.push(Err(ToolParseError {
                    raw: line.to_string(),
                    reason: "invalid JSON tool call or unknown tool name".into(),
                }));
                continue;
            }
        }
        if line.contains("[TOOL_CALL]") {
            match parse_tool_call_markup(line) {
                Some(t) => results.push(Ok(t)),
                None => results.push(Err(ToolParseError {
                    raw: line.to_string(),
                    reason: "invalid [TOOL_CALL] markup or unknown tool name".into(),
                })),
            }
            continue;
        }
        // Inline legacy marker anywhere on the line (e.g. trailing TOOL:name:args).
        for (idx, _) in line.match_indices("TOOL:") {
            if idx == 0 {
                continue;
            }
            if let Some(t) = parse_legacy_tool(&line[idx + 5..]) {
                results.push(Ok(t));
            }
        }
    }
    results
}

/// Build a tool-result message for a parse error.
pub fn tool_parse_error_message(error: &ToolParseError, id: &str) -> ChatMessage {
    ChatMessage::tool_result(format!(
        "Could not parse tool call: {}. Raw input: {}",
        error.reason, error.raw
    ))
    .with_tool_call_id(id)
}

/// Check if text contains tool call markers.
pub fn has_tool_calls(text: &str) -> bool {
    if text.contains(TOOL_CALL_START) && text.contains(TOOL_CALL_END) {
        return true;
    }
    if minimax::has_minimax_tool_calls(text) {
        return true;
    }
    text.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with("TOOL:")
            || (trimmed.starts_with('{')
                && serde_json::from_str::<serde_json::Value>(trimmed)
                    .ok()
                    .is_some_and(|v| is_tool_call_value(&v)))
        {
            return true;
        }
        // Inline legacy marker (e.g. "...directory.TOOL:list_dir:.")
        line.match_indices("TOOL:").any(|(idx, _)| {
            parse_legacy_tool(&line[idx + 5..]).is_some()
        })
    })
}
// Parsers

fn parse_legacy_tool(payload: &str) -> Option<ParsedToolCall> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Accept both colon-separated (TOOL:bash:ls) and whitespace-separated
    // (TOOL:bash ls) legacy forms.
    let (tool_name, arg1, arg2): (&str, String, String) = if trimmed.contains(':') {
        let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
        (
            parts[0],
            parts.get(1).unwrap_or(&"").to_string(),
            parts.get(2).unwrap_or(&"").to_string(),
        )
    } else {
        let mut tokens = trimmed.split_whitespace();
        let name = tokens.next().unwrap_or("");
        let first = tokens.next().unwrap_or("").to_string();
        let rest = tokens.collect::<Vec<_>>().join(" ");
        (name, first, rest)
    };

    if tool_name.is_empty() {
        return None;
    }

    let mut args = Map::new();
    match tool_name {
        "read_file" => {
            args.insert("path".to_string(), Value::String(arg1));
        }
        "list_dir" => {
            args.insert("path".to_string(), Value::String(arg1));
        }
        "write_file" => {
            args.insert("path".to_string(), Value::String(arg1));
            args.insert("content".to_string(), Value::String(arg2));
        }
        "bash" => {
            args.insert("command".to_string(), Value::String(arg1));
        }
        _ => return None,
    }
    Some(ParsedToolCall {
        name: tool_name.to_string(),
        args: Value::Object(args),
        id: None,
    })
}

fn parse_inline_json_tools(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
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

fn parse_tool_call_markup(line: &str) -> Option<ParsedToolCall> {
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

fn parse_minimax_tool_calls(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    minimax::parse_minimax_tool_calls(text)
}

pub fn is_tool_call_value(value: &Value) -> bool {
    value.get("name").is_some()
        && value.get("arguments").is_some()
        && value
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(is_known_tool)
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

/// Assign synthetic ids to parsed tool calls.
pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for (i, tool) in tools.iter_mut().enumerate() {
        if tool.id.is_none() {
            tool.id = Some(format!("call_{}", i));
        }
    }
}

pub fn build_assistant_message(
    response_text: &str,
    reasoning: Option<&str>,
    tools: &[ParsedToolCall],
) -> ChatMessage {
    let tool_calls: Vec<ToolCall> = tools
        .iter()
        .map(|t| {
            ToolCall::new(
                t.id.clone().unwrap_or_default(),
                &t.name,
                t.args.clone(),
            )
        })
        .collect();
    let mut parts = Vec::with_capacity(tools.len() + 2);
    if !response_text.is_empty() {
        parts.push(Part::text(response_text));
    }
    if let Some(reasoning) = reasoning {
        if !reasoning.is_empty() {
            parts.push(Part::reasoning(reasoning));
        }
    }
    for tool in tools {
        parts.push(Part::tool_call(
            tool.id.clone().unwrap_or_default(),
            &tool.name,
            tool.args.clone(),
        ));
    }
    ChatMessage::assistant(response_text.to_string())
        .with_tool_calls(tool_calls)
        .with_parts(parts)
}

#[cfg(test)]
mod tests;
