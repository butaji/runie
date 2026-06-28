//! MiniMax XML tool-call parsing via `quick-xml`.
//!
//! Handles both M2 (</minimax:tool_call>) and M3 (<tool_call>) formats.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};
use super::{ParsedToolCall, ToolParseError};
use crate::tool::BUILTIN_TOOL_NAMES;

pub const OPEN_M2: &str = "<invoke";  // M2 open marker
pub const CLOSE_M2: &str = "</minimax:tool_call>";  // M2 close marker
pub const OPEN_M3: &str = "<tool_call>";
pub const CLOSE_M3: &str = "</tool_call>";

/// Protocol-level tool names recognized by the parser but not implemented as full tools.
/// These are signals/messages in the MiniMax protocol, not MCP tools.
const PROTOCOL_TOOL_NAMES: &[&str] = &[
    "ask_user", "select_model", "done",
];

/// All known tool names for validation (canonical built-ins + protocol names).
fn known_tools() -> Vec<&'static str> {
    BUILTIN_TOOL_NAMES
        .iter()
        .chain(PROTOCOL_TOOL_NAMES.iter())
        .copied()
        .collect()
}

/// Check if a tool name is known (built-in or protocol-level).
pub fn is_known_tool(name: &str) -> bool {
    known_tools().contains(&name)
}

pub fn parse_minimax_tool_calls(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let norm = normalize_m3(text);
    if let Some(b) = extract_block(&norm, OPEN_M2, CLOSE_M2) {
        parse_invoke_blocks(b)
    } else if let Some(b) = extract_block(&norm, OPEN_M3, CLOSE_M3) {
        parse_invoke_blocks(b)
    } else {
        Vec::new()
    }
}

fn normalize_m3(text: &str) -> String {
    text.replace("]<]minimax[>[</", "</").replace("]<]minimax[>[<", "<")
}

fn extract_block<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?;
    let after = &text[start..];
    let end = after[1..].find(close)?;
    Some(&after[..=end])
}

/// Parse all `<invoke>` blocks inside the extracted wrapper.
fn parse_invoke_blocks(block: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut reader = Reader::from_str(block);
    reader.config_mut().trim_text(true);
    let mut results = Vec::new();
    let mut body_start: Option<usize> = None;
    let mut name: Option<String> = None;
    let mut pos: usize;
    loop {
        pos = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let tag = xml_tag_name(e.name());
                if tag == "invoke" {
                    // pos is the position of '<' in <invoke ...>: body starts after '>'.
                    body_start = Some(pos + e.len() + 1);
                    name = e.attributes().flatten()
                        .find(|a| String::from_utf8_lossy(a.key.as_ref()).as_ref() == "name")
                        .map(|a| String::from_utf8_lossy(&a.value).into_owned());
                }
            }
            Ok(Event::End(ref e)) => {
                if xml_tag_name(e.name()) == "invoke" {
                    if let (Some(start), Some(n)) = (body_start, name.take()) {
                        // pos is the position of '<' in </invoke>: exclusive end is before it.
                        let end = pos;
                        flush_invoke(&mut results, block, start, end, n);
                    }
                    body_start = None;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
    if results.is_empty() {
        results.push(Err(ToolParseError { raw: block.to_owned(), reason: "no <invoke> blocks found".into() }));
    }
    results
}

fn xml_tag_name<B: AsRef<[u8]>>(name: B) -> String {
    String::from_utf8_lossy(name.as_ref()).into_owned()
}

fn flush_invoke(
    results: &mut Vec<Result<ParsedToolCall, ToolParseError>>,
    block: &str,
    start: usize,
    end: usize,
    name: String,
) {
    let body = &block[start..end];
    let args = parse_body(body);
    results.push(if is_known_tool(&name) {
        Ok(ParsedToolCall { name, args: Value::Object(args), id: None })
    } else {
        Err(ToolParseError { raw: block.to_owned(), reason: format!("unknown tool '{}'", name) })
    });
}

fn parse_body(body: &str) -> Map<String, Value> {
    let mut args = Map::new();
    let mut rest = body;
    let mut found_param = false;
    while let Some(start) = rest.find("<parameter") {
        let after = &rest[start + "<parameter".len()..];
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];
        let name = extract_attr(tag, "name");
        let after_tag = &after[close + 1..];
        let Some(end) = after_tag.find("</parameter>") else { break };
        let val_str = after_tag[..end].trim();
        let val = serde_json::from_str(val_str).unwrap_or(Value::String(val_str.to_owned()));
        if let Some(n) = name { args.insert(n, val); found_param = true; }
        rest = &after_tag[end + "</parameter>".len()..];
    }
    if found_param { return args; }
    parse_child_tags(rest.trim_end(), &mut args);
    args
}

fn parse_child_tags(inner: &str, args: &mut Map<String, Value>) {
    let mut rest = inner;
    while let Some(start) = rest.find("<") {
        let after = &rest[start + 1..];
        let Some(close) = after.find('>') else { break };
        let tag = after[..close].trim();
        if tag.starts_with('/') { rest = &after[close + 1..]; continue; }
        let name = tag.split_whitespace().next().unwrap_or(tag);
        let after_tag = &after[close + 1..];
        let end_tag = format!("</{}>", name);
        let Some(end) = after_tag.find(&end_tag) else { break };
        let val_str = after_tag[..end].trim();
        let val = serde_json::from_str(val_str).unwrap_or(Value::String(val_str.to_owned()));
        args.insert(name.to_owned(), val);
        rest = &after_tag[end + end_tag.len()..];
    }
}

fn extract_attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{}=", key);
    let mut rest = tag;
    while let Some(idx) = rest.find(&pat) {
        let after = &rest[idx + pat.len()..];
        let quote = after.chars().next()?;
        if quote != '\'' && quote != '"' { rest = &after[1..]; continue; }
        let after_q = &after[1..];
        let end = after_q.find(quote)?;
        return Some(after_q[..end].to_string());
    }
    None
}
