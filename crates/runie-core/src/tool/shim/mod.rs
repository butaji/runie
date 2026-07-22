//!
//! Thin shim for text-based tool-call parsing.
//!
//! Uses `quick-xml` for MiniMax XML tool calls and a single-pass JSON object
//! detector. Replaces the legacy parser stack.

pub mod json;
pub mod minimax;

pub use json::{is_tool_call_value, is_tool_call_value_check, parse_inline_json_tools};
pub use minimax::{is_known_tool, parse_minimax_tool_calls, OPEN_M2, OPEN_M3};

use super::types::{ParsedToolCall, ToolParseError};
use serde_json::{Map, Value};

const TC_START: &str = "[TOOL_CALL]";
const TC_END: &str = "[/TOOL_CALL]";

// ─── Legacy TOOL: parser ──────────────────────────────────────────────────────

pub(crate) fn parse_legacy_tools_in_line(line: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("TOOL:") {
        match parse_legacy_tool(rest) {
            Some(t) => results.push(Ok(t)),
            None if !rest.trim().is_empty() => {
                results.push(Err(ToolParseError {
                    raw: line.to_owned(),
                    reason: "invalid legacy TOOL syntax".into(),
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

pub(crate) fn parse_legacy_tool(payload: &str) -> Option<ParsedToolCall> {
    let t = payload.trim();
    if t.is_empty() {
        return None;
    }
    let (name, a1, a2) = if t.contains(':') {
        parse_legacy_colon(t)
    } else {
        parse_legacy_space(t)
    };
    if name.is_empty() {
        return None;
    }
    let args = build_legacy_args(name, a1, a2)?;
    Some(ParsedToolCall { name: name.to_owned(), args, id: None })
}

fn parse_legacy_colon(t: &str) -> (&str, String, String) {
    let p: Vec<&str> = t.splitn(3, ':').collect();
    (
        p[0],
        p.get(1).unwrap_or(&"").to_string(),
        p.get(2).unwrap_or(&"").to_string(),
    )
}

fn parse_legacy_space(t: &str) -> (&str, String, String) {
    let mut tok = t.split_whitespace();
    let name = tok.next().unwrap_or("");
    let first = tok.next().unwrap_or("").to_owned();
    let rest = tok.collect::<Vec<_>>().join(" ");
    (name, first, rest)
}

fn build_legacy_args(name: &str, a1: String, a2: String) -> Option<Value> {
    let mut args = Map::new();
    match name {
        "read_file" | "list_dir" => {
            args.insert("path".into(), Value::String(a1));
        }
        "write_file" => {
            args.insert("path".into(), Value::String(a1));
            args.insert("content".into(), Value::String(a2));
        }
        "bash" => {
            args.insert("command".into(), Value::String(a1));
        }
        _ => return None,
    }
    Some(Value::Object(args))
}

// ─── Arrow-to-JSON normalizer for [TOOL_CALL] markup ───────────────────────────

#[allow(clippy::too_many_lines)]
fn arrow_to_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            out.push('"');
            continue;
        }
        if !in_str(&out) && ch == '=' && chars.peek() == Some(&'>') {
            chars.next();
            out.push(':');
            if chars.peek() == Some(&' ') {
                chars.next();
                out.push(' ');
            }
            continue;
        }
        if !in_str(&out) && (ch.is_alphabetic() || ch == '_') {
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

fn in_str(s: &str) -> bool {
    s.matches('"').count() % 2 == 1
}

// ─── Unified non-XML tool call parser ───────────────────────────────────────
// Tries inline JSON, then [TOOL_CALL] markup (arrow syntax), then legacy TOOL:.
// Returns the first successful parse, or None if no format matches.

fn try_parse_non_xml_tool(line: &str) -> Option<ParsedToolCall> {
    // 1. Inline JSON: {"name":"...","arguments":{...}}
    let inline = parse_inline_json_tools(line);
    if !inline.is_empty() {
        return inline.into_iter().find_map(|r| r.ok());
    }

    // 2. [TOOL_CALL]{...}[/TOOL_CALL] markup (arrow syntax)
    if let Some(payload) = extract_tool_call_payload(line) {
        let json = arrow_to_json(payload);
        let v: Value = serde_json::from_str(&json).ok()?;
        let name = v.get("tool")?.as_str()?;
        let args = v.get("args")?.as_object()?;
        if is_known_tool(name) {
            return Some(ParsedToolCall { name: name.to_owned(), args: Value::Object(args.clone()), id: None });
        }
    }

    // 3. Legacy TOOL:name:arg1:arg2
    parse_legacy_tool(line)
}

fn extract_tool_call_payload(line: &str) -> Option<&str> {
    let start = line.find("[TOOL_CALL]")?;
    let after = &line[start + "[TOOL_CALL]".len()..];
    let end = after.find("[/TOOL_CALL]")?;
    Some(after[..end].trim())
}

pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    parse_tool_calls_fallible(text)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn parse_tool_calls_fallible(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let minimax = parse_minimax_tool_calls(text);
    if !minimax.is_empty() {
        return minimax;
    }
    let mut results = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        results.extend(parse_line_strategies(trimmed, line));
    }
    results
}

fn parse_line_strategies(trimmed: &str, original: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    // If the line looks like JSON (has '{'), try JSON first.
    // If it looks like markup (has [TOOL_CALL]), try it last.
    // Return errors for tool-like-but-invalid inputs so callers can report them.
    let looks_like_json = trimmed.contains('{');
    let looks_like_markup = trimmed.contains(TC_START);

    if looks_like_json {
        if let Some(tool) = try_parse_non_xml_tool(trimmed) {
            return vec![Ok(tool)];
        }
        // Looks like JSON but failed to parse — return an error.
        return vec![Err(ToolParseError { raw: original.to_owned(), reason: "invalid JSON".into() })];
    }

    if looks_like_markup {
        // Markup found but failed to parse — return an error.
        if let Some(tool) = try_parse_non_xml_tool(trimmed) {
            return vec![Ok(tool)];
        }
        return vec![Err(ToolParseError {
            raw: original.to_owned(),
            reason: "invalid [TOOL_CALL] markup or unknown tool name".into(),
        })];
    }

    // Fall back to the legacy multi-tool parser.
    parse_legacy_tools_in_line(trimmed)
}

pub fn has_tool_calls(text: &str) -> bool {
    if text.contains(TC_START) && text.contains(TC_END) {
        return true;
    }
    if text.contains(OPEN_M2) || text.contains(OPEN_M3) {
        return true;
    }
    text.lines().any(has_tool_calls_in_line)
}

fn has_tool_calls_in_line(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with("TOOL:") {
        return true;
    }
    if t.starts_with('{') && is_tool_call_value_check(t) {
        return true;
    }
    line.match_indices("TOOL:")
        .any(|(idx, _)| parse_legacy_tool(&line[idx + 5..]).is_some())
}

pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for tool in tools {
        if tool.id.is_none() {
            tool.id = Some("call_0".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimax_m3() {
        let t = r#"<tool_call><invoke name="read_file"><parameter name="path">README.md</parameter></invoke>
</minimax:tool_call>"#;
        let r = parse_tool_calls_fallible(t);
        assert!(!r.is_empty());
        assert_eq!(r[0].as_ref().unwrap().name, "read_file");
    }
    #[test]
    fn inline_json() {
        let t = r#"{"name":"bash","arguments":{"command":"ls"}}"#;
        assert!(!parse_tool_calls_fallible(t).is_empty());
    }
    #[test]
    fn legacy_colon() {
        let r = parse_tool_calls_fallible("TOOL:bash:ls");
        assert!(!r.is_empty());
        assert_eq!(r[0].as_ref().unwrap().name, "bash");
    }
    #[test]
    fn markup() {
        let t = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#;
        assert!(!parse_tool_calls_fallible(t).is_empty());
    }
    #[test]
    fn minimax_m2_with_param() {
        let t = r#"<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>"#;
        let r = parse_tool_calls_fallible(t);
        assert!(!r.is_empty(), "expected at least one tool call");
        let tool = r[0].as_ref().unwrap();
        assert_eq!(tool.name, "list_dir");
        eprintln!("DEBUG args: {:?}", tool.args);
        assert_eq!(
            tool.args.get("path").map(|v| v.as_str().unwrap()),
            Some("."),
            "path should be '.'"
        );
    }

    #[test]
    fn debug_buffer_positions() {
        use quick_xml::events::Event;
        use quick_xml::Reader;
        let block = "<invoke name=\"list_dir\">\n<parameter name=\"path\">.</parameter>\n</invoke>\n";
        let mut reader = Reader::from_str(block);
        reader.config_mut().trim_text(true);
        loop {
            let pos = reader.buffer_position();
            match reader.read_event() {
                Ok(e) => {
                    eprintln!("Event: {:?}, pos={}", e, pos);
                    if matches!(e, Event::Eof) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}
