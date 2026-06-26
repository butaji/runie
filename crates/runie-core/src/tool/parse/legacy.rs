//! Legacy `TOOL:` tool-call format parser.

use serde_json::{Map, Value};

use super::{ParsedToolCall, ToolParseError};

pub fn parse_legacy_tools_in_line(line: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("TOOL:") {
        match parse_legacy_tool(rest) {
            Some(t) => results.push(Ok(t)),
            None if !rest.trim().is_empty() => {
                results.push(Err(ToolParseError {
                    raw: line.to_owned(),
                    reason: "invalid legacy TOOL syntax or unknown tool name".into(),
                }));
            }
            None => {}
        }
    }
    for (idx, _) in line.match_indices("TOOL:") {
        if idx == 0 {
            continue;
        }
        if let Some(t) = parse_legacy_tool(&line[idx + 5..]) {
            results.push(Ok(t));
        }
    }
    results
}

pub fn parse_legacy_tool(payload: &str) -> Option<ParsedToolCall> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (tool_name, arg1, arg2) = if trimmed.contains(':') {
        parse_legacy_colon_form(trimmed)
    } else {
        parse_legacy_space_form(trimmed)
    };
    if tool_name.is_empty() {
        return None;
    }
    let args = build_legacy_args(tool_name, arg1, arg2)?;
    Some(ParsedToolCall {
        name: tool_name.to_owned(),
        args,
        id: None,
    })
}

fn parse_legacy_colon_form(trimmed: &str) -> (&str, String, String) {
    let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
    (
        parts[0],
        parts.get(1).unwrap_or(&"").to_string(),
        parts.get(2).unwrap_or(&"").to_string(),
    )
}

fn parse_legacy_space_form(trimmed: &str) -> (&str, String, String) {
    let mut tokens = trimmed.split_whitespace();
    let name = tokens.next().unwrap_or("");
    let first = tokens.next().unwrap_or("").to_owned();
    let rest = tokens.collect::<Vec<_>>().join(" ");
    (name, first, rest)
}

pub fn build_legacy_args(tool_name: &str, arg1: String, arg2: String) -> Option<Value> {
    let mut args = Map::new();
    match tool_name {
        "read_file" => {
            args.insert("path".to_owned(), Value::String(arg1));
        }
        "list_dir" => {
            args.insert("path".to_owned(), Value::String(arg1));
        }
        "write_file" => {
            args.insert("path".to_owned(), Value::String(arg1));
            args.insert("content".to_owned(), Value::String(arg2));
        }
        "bash" => {
            args.insert("command".to_owned(), Value::String(arg1));
        }
        _ => return None,
    }
    Some(Value::Object(args))
}
