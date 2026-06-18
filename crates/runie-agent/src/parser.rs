//! Tool call parser — extracts tool invocations from LLM text output.
//!
//! Returns `(name, arguments)` tuples for use with `ToolRegistry`.

use runie_core::message::{ChatMessage, Part, ToolCall};
use serde_json::{Map, Value};

const TOOL_CALL_START: &str = "[TOOL_CALL]";
const TOOL_CALL_END: &str = "[/TOOL_CALL]";

/// A parsed tool invocation: name, JSON arguments, and an optional call id.
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: Value,
    pub id: Option<String>,
}

/// A tool-call parse error: the raw line and a human-readable reason.
#[derive(Debug, Clone)]
pub struct ToolParseError {
    pub raw: String,
    pub reason: String,
}

/// Parse tool calls from LLM text output.
/// Returns a list of `(tool_name, arguments)` tuples.
pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    parse_tool_calls_fallible(text)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

/// Parse tool calls, returning both successful parses and malformed lines.
///
/// Malformed lines are surfaced to the caller instead of being silently
/// dropped, so the agent loop can feed the error back to the model.
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
        if line.starts_with("TOOL:") {
            match parse_legacy_tool(line.strip_prefix("TOOL:").unwrap_or("")) {
                Some(t) => results.push(Ok(t)),
                None => results.push(Err(ToolParseError {
                    raw: line.to_string(),
                    reason: "invalid legacy TOOL syntax or unknown tool name".into(),
                })),
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
        }
    }
    results
}

/// Build a tool-result message that tells the model a tool call could not be
/// parsed. This lets the model self-correct on the next turn.
pub fn tool_parse_error_message(error: &ToolParseError, id: &str) -> ChatMessage {
    ChatMessage::tool_result(format!(
        "Could not parse tool call: {}. Raw input: {}",
        error.reason, error.raw
    ))
    .with_tool_call_id(id)
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
    const OPEN: &str = "<minimax:tool_call>";
    const CLOSE: &str = "</minimax:tool_call>";
    let mut results = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            results.push(Err(ToolParseError {
                raw: rest[start..].to_string(),
                reason: "unclosed <minimax:tool_call> block".into(),
            }));
            break;
        };
        let block = &after_open[..end];
        results.extend(parse_minimax_invokes(block));
        rest = &after_open[end + CLOSE.len()..];
    }
    results
}

fn parse_minimax_invokes(block: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let mut found_any = false;
    let mut rest = block;
    while let Some(start) = rest.find("<invoke") {
        found_any = true;
        let after_tag_open = &rest[start + "<invoke".len()..];
        let Some(close) = after_tag_open.find('>') else {
            results.push(Err(ToolParseError {
                raw: rest[start..].to_string(),
                reason: "unclosed <invoke> tag".into(),
            }));
            break;
        };
        let tag = &after_tag_open[..close];
        let Some(name) = extract_minimax_name_attr(tag) else {
            results.push(Err(ToolParseError {
                raw: rest[start..].to_string(),
                reason: "missing name attribute on <invoke>".into(),
            }));
            break;
        };
        let after_tag = &after_tag_open[close + 1..];
        let Some(invoke_end) = after_tag.find("</invoke>") else {
            results.push(Err(ToolParseError {
                raw: rest[start..].to_string(),
                reason: "missing </invoke> closing tag".into(),
            }));
            break;
        };
        let inner = &after_tag[..invoke_end];
        rest = &after_tag[invoke_end + "</invoke>".len()..];
        if !is_known_tool(&name) {
            results.push(Err(ToolParseError {
                raw: block.to_string(),
                reason: format!("unknown tool '{}'", name),
            }));
            continue;
        }
        let args = parse_minimax_parameters(inner);
        results.push(Ok(ParsedToolCall {
            name,
            args: Value::Object(args),
            id: None,
        }));
    }
    if !found_any {
        results.push(Err(ToolParseError {
            raw: block.to_string(),
            reason: "no <invoke> blocks found in <minimax:tool_call>".into(),
        }));
    }
    results
}

fn extract_minimax_name_attr(tag: &str) -> Option<String> {
    extract_xml_attr(tag, "name")
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

fn parse_minimax_parameters(inner: &str) -> serde_json::Map<String, Value> {
    let mut args = serde_json::Map::new();
    let mut rest = inner;
    while let Some(start) = rest.find("<parameter") {
        let after_open = &rest[start + "<parameter".len()..];
        let Some(close) = after_open.find('>') else {
            break;
        };
        let tag = &after_open[..close];
        let Some(name) = extract_xml_attr(tag, "name") else {
            rest = &after_open[close + 1..];
            continue;
        };
        let after_tag = &after_open[close + 1..];
        let Some(end) = after_tag.find("</parameter>") else {
            break;
        };
        let value_str = after_tag[..end].trim();
        let value = serde_json::from_str(value_str).unwrap_or(Value::String(value_str.to_string()));
        args.insert(name, value);
        rest = &after_tag[end + "</parameter>".len()..];
    }
    args
}

fn is_known_tool(name: &str) -> bool {
    const KNOWN: &[&str] = &[
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
    KNOWN.contains(&name)
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

/// Assign synthetic ids to parsed tool calls when the provider did not supply
/// them (e.g. prompt-based tool markers).
pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for (i, tool) in tools.iter_mut().enumerate() {
        if tool.id.is_none() {
            tool.id = Some(format!("call_{}", i));
        }
    }
}

/// Build an assistant `ChatMessage` that carries the raw response text,
/// optional reasoning, and first-class `ToolCall` objects parsed from it.
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
                t.args.to_string(),
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


