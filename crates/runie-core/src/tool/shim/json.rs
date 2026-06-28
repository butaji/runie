//! JSON tool-call detection — single-pass, no regex.

use serde_json::Value;
use super::{is_known_tool, ParsedToolCall, ToolParseError};

pub fn is_tool_call_value(v: &Value) -> bool {
    v.get("name").and_then(|x| x.as_str()).is_some_and(is_known_tool)
        && v.get("arguments").is_some()
}

pub fn is_tool_call_value_check(line: &str) -> bool {
    serde_json::from_str::<Value>(line).ok().is_some_and(|v| is_tool_call_value(&v))
}

pub fn find_json_tool_calls(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut results = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = find_object_end(bytes, i) {
                if let Ok(slice) = std::str::from_utf8(&bytes[i..=end]) {
                    if let Ok(v) = serde_json::from_str::<Value>(slice) {
                        if is_tool_call_value(&v) {
                            results.push((i, end));
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    results
}

pub fn find_object_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_str = false;
    let mut esc = false;
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            esc = esc || c == b'\\';
            if c == b'"' && !esc { in_str = false; esc = false; }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' { depth += 1; }
        else if c == b'}' { depth -= 1; if depth == 0 { return Some(i); } }
        i += 1;
    }
    None
}

pub fn parse_inline_json_tools(line: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    find_json_tool_calls(line).into_iter().filter_map(|(s, e)| {
        let slice = &line[s..=e];
        serde_json::from_str::<Value>(slice).ok().and_then(|v| {
            let n = v.get("name")?.as_str()?;
            let a = v.get("arguments")?.as_object()?;
            if !is_known_tool(n) { return None; }
            Some(Ok(ParsedToolCall { name: n.to_owned(), args: Value::Object(a.clone()), id: None }))
        })
    }).collect()
}
