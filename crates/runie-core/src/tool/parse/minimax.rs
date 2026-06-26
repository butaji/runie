//! MiniMax XML tool-call format parser.

use serde_json::{Map, Value};

use super::{ParsedToolCall, ToolParseError};

pub const OPEN_M2: &str = "<minimax:tool_call>";
pub const CLOSE_M2: &str = "</minimax:tool_call>";
pub const OPEN_M3: &str = "<tool_call>";
pub const CLOSE_M3: &str = "</tool_call>";

/// Known tool names for validation.
pub const KNOWN_TOOLS: &[&str] = &[
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
];

pub fn is_known_tool(name: &str) -> bool {
    KNOWN_TOOLS.contains(&name)
}

pub fn parse_minimax_tool_calls(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let normalized = normalize_m3_delimiters(text);
    let block = extract_block(&normalized, OPEN_M2, CLOSE_M2)
        .or_else(|| extract_block(&normalized, OPEN_M3, CLOSE_M3));
    let block = match block {
        Some(b) => b,
        None => return Vec::new(),
    };
    parse_minimax_invokes(block)
}

pub fn normalize_m3_delimiters(text: &str) -> String {
    let mut out = text.to_owned();
    out = out.replace("]<]minimax[>[</", "</");
    out = out.replace("]<]minimax[>[<", "<");
    out
}

fn extract_block<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?;
    let after_open = &text[start + open.len()..];
    let end = after_open.find(close)?;
    Some(&after_open[..end])
}

pub fn parse_minimax_invokes(block: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    let mut found_any = false;
    let mut rest = block;
    while let Some(result) = parse_next_invoke(&mut rest, block) {
        found_any = true;
        results.push(result);
    }
    if !found_any {
        results.push(Err(ToolParseError {
            raw: block.to_owned(),
            reason: "no <invoke> blocks found in tool_call block".into(),
        }));
    }
    results
}

fn parse_next_invoke(
    rest: &mut &str,
    block: &str,
) -> Option<Result<ParsedToolCall, ToolParseError>> {
    let start = rest.find("<invoke")?;
    let after_tag_open = &rest[start + "<invoke".len()..];
    let close = after_tag_open.find('>')?;
    let tag = &after_tag_open[..close];
    let name = extract_xml_attr(tag, "name")?;
    let after_tag = &after_tag_open[close + 1..];
    let invoke_end = after_tag.find("</invoke>")?;
    let inner = &after_tag[..invoke_end];
    *rest = &after_tag[invoke_end + "</invoke>".len()..];
    if !is_known_tool(&name) {
        return Some(Err(ToolParseError {
            raw: block.to_owned(),
            reason: format!("unknown tool '{}'", name),
        }));
    }
    let args = parse_minimax_parameters(inner);
    Some(Ok(ParsedToolCall {
        name,
        args: Value::Object(args),
        id: None,
    }))
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

pub fn parse_minimax_parameters(inner: &str) -> Map<String, Value> {
    let mut args = Map::new();
    let mut rest = inner;
    let mut found_parameter = false;
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
        let value =
            serde_json::from_str(value_str).unwrap_or(Value::String(value_str.to_owned()));
        args.insert(name, value);
        found_parameter = true;
        rest = &after_tag[end + "</parameter>".len()..];
    }
    if found_parameter {
        return args;
    }
    parse_minimax_child_tags(inner, &mut args);
    args
}

fn parse_minimax_child_tags(inner: &str, args: &mut Map<String, Value>) {
    let mut rest = inner;
    while let Some(start) = rest.find('<') {
        let after_open = &rest[start + 1..];
        let Some(close) = after_open.find('>') else {
            break;
        };
        let tag = &after_open[..close];
        if tag.starts_with('/') {
            rest = &after_open[close + 1..];
            continue;
        }
        let name = tag.split_whitespace().next().unwrap_or(tag);
        let after_tag = &after_open[close + 1..];
        let end_tag = format!("</{}>", name);
        let Some(end) = after_tag.find(&end_tag) else {
            break;
        };
        let value_str = after_tag[..end].trim();
        let value =
            serde_json::from_str(value_str).unwrap_or(Value::String(value_str.to_owned()));
        args.insert(name.to_owned(), value);
        rest = &after_tag[end + end_tag.len()..];
    }
}
