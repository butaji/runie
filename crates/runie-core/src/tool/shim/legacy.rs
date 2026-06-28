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
                results.push(Err(ToolParseError { raw: line.to_owned(), reason: "invalid legacy TOOL syntax".into() }));
            }
            None => {}
        }
    }
    for (idx, _) in line.match_indices("TOOL:") {
        if idx == 0 { continue; }
        if let Some(t) = parse_legacy_tool(&line[idx + 5..]) { results.push(Ok(t)); }
    }
    results
}

pub fn parse_legacy_tool(payload: &str) -> Option<ParsedToolCall> {
    let t = payload.trim();
    if t.is_empty() { return None; }
    let (name, a1, a2) = if t.contains(':') { parse_colon(t) } else { parse_space(t) };
    if name.is_empty() { return None; }
    let args = build_legacy_args(name, a1, a2)?;
    Some(ParsedToolCall { name: name.to_owned(), args, id: None })
}

fn parse_colon(t: &str) -> (&str, String, String) {
    let p: Vec<&str> = t.splitn(3, ':').collect();
    (p[0], p.get(1).unwrap_or(&"").to_string(), p.get(2).unwrap_or(&"").to_string())
}

fn parse_space(t: &str) -> (&str, String, String) {
    let mut tok = t.split_whitespace();
    let name = tok.next().unwrap_or("");
    let first = tok.next().unwrap_or("").to_owned();
    let rest = tok.collect::<Vec<_>>().join(" ");
    (name, first, rest)
}

pub fn build_legacy_args(name: &str, a1: String, a2: String) -> Option<Value> {
    let mut args = Map::new();
    match name {
        "read_file" | "list_dir" => { args.insert("path".into(), Value::String(a1)); }
        "write_file" => { args.insert("path".into(), Value::String(a1)); args.insert("content".into(), Value::String(a2)); }
        "bash" => { args.insert("command".into(), Value::String(a1)); }
        _ => return None,
    }
    Some(Value::Object(args))
}
