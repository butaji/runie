//! `[TOOL_CALL]` markup parser.

use serde_json::Value;
use super::{is_known_tool, ParsedToolCall};

pub fn parse_markup_tool(line: &str) -> Option<ParsedToolCall> {
    let payload = extract_tool_call_payload(line)?;
    let json = arrow_to_json(payload);
    let v: Value = serde_json::from_str(&json).ok()?;
    let name = v.get("tool")?.as_str()?;
    let args = v.get("args")?.as_object()?;
    if !is_known_tool(name) { return None; }
    Some(ParsedToolCall { name: name.to_owned(), args: Value::Object(args.clone()), id: None })
}

pub fn extract_tool_call_payload(line: &str) -> Option<&str> {
    let start = line.find("[TOOL_CALL]")?;
    let after = &line[start + "[TOOL_CALL]".len()..];
    let end = after.find("[/TOOL_CALL]")?;
    Some(after[..end].trim())
}

pub fn arrow_to_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' { out.push('"'); continue; }
        if !in_str(&out) && ch == '=' && chars.peek() == Some(&'>') {
            chars.next();
            out.push(':');
            if chars.peek() == Some(&' ') { chars.next(); out.push(' '); }
            continue;
        }
        if !in_str(&out) && (ch.is_alphabetic() || ch == '_') {
            let mut word = String::new();
            word.push(ch);
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' { word.push(c); chars.next(); }
                else { break; }
            }
            let last = out.trim_end().chars().last();
            let is_key = last == Some('{') || last == Some(',');
            if is_key { out.push('"'); out.push_str(&word); out.push('"'); }
            else { out.push_str(&word); }
            continue;
        }
        out.push(ch);
    }
    out
}

fn in_str(s: &str) -> bool {
    s.matches('"').count() % 2 == 1
}
