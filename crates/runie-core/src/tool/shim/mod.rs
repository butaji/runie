//!
//! Thin shim for text-based tool-call parsing.
//!
//! Uses `quick-xml` for MiniMax XML tool calls and a single-pass JSON object
//! detector. Replaces the legacy parser stack.

pub mod json;
pub mod legacy;
pub mod markup;
pub mod minimax;

pub use json::{is_tool_call_value, is_tool_call_value_check, parse_inline_json_tools};
pub use legacy::{build_legacy_args, parse_legacy_tool, parse_legacy_tools_in_line};
pub use markup::{arrow_to_json, extract_tool_call_payload, parse_markup_tool};
pub use minimax::{
    is_known_tool, parse_minimax_tool_calls, OPEN_M2, OPEN_M3,
};

use super::types::{ParsedToolCall, ToolParseError};

const TC_START: &str = "[TOOL_CALL]";
const TC_END: &str = "[/TOOL_CALL]";

pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    parse_tool_calls_fallible(text).into_iter().filter_map(|r| r.ok()).collect()
}

pub fn parse_tool_calls_fallible(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let minimax = parse_minimax_tool_calls(text);
    if !minimax.is_empty() { return minimax; }
    let mut results = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        results.extend(parse_line_strategies(trimmed, line));
    }
    results
}

fn parse_line_strategies(trimmed: &str, original: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    results.extend(parse_legacy_tools_in_line(trimmed));
    if trimmed.contains('{') {
        let inline = parse_inline_json_tools(trimmed);
        if !inline.is_empty() {
            results.extend(inline);
        } else if trimmed.starts_with('{') {
            results.push(Err(ToolParseError { raw: original.to_owned(), reason: "invalid JSON".into() }));
        }
    }
    if trimmed.contains(TC_START) {
        match parse_markup_tool(trimmed) {
            Some(t) => results.push(Ok(t)),
            None => results.push(Err(ToolParseError { raw: original.to_owned(), reason: "invalid [TOOL_CALL] markup or unknown tool name".into() })),
        }
    }
    results
}

pub fn has_tool_calls(text: &str) -> bool {
    if text.contains(TC_START) && text.contains(TC_END) { return true; }
    if text.contains(OPEN_M2) || text.contains(OPEN_M3) { return true; }
    text.lines().any(has_tool_calls_in_line)
}

fn has_tool_calls_in_line(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with("TOOL:") { return true; }
    if t.starts_with('{') && is_tool_call_value_check(t) { return true; }
    line.match_indices("TOOL:").any(|(idx, _)| parse_legacy_tool(&line[idx + 5..]).is_some())
}

pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for tool in tools { if tool.id.is_none() { tool.id = Some("call_0".into()); } }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn minimax_m3() {
        let t = r#"<tool_call><invoke name="read_file"><parameter name="path">README.md</parameter></invoke>
</minimax:tool_call>"#;
        let r = parse_tool_calls_fallible(t);
        assert!(!r.is_empty());
        assert_eq!(r[0].as_ref().unwrap().name, "read_file");
    }
    #[test] fn inline_json() {
        let t = r#"{"name":"bash","arguments":{"command":"ls"}}"#;
        assert!(!parse_tool_calls_fallible(t).is_empty());
    }
    #[test] fn legacy_colon() {
        let r = parse_tool_calls_fallible("TOOL:bash:ls");
        assert!(!r.is_empty());
        assert_eq!(r[0].as_ref().unwrap().name, "bash");
    }
    #[test] fn markup() {
        let t = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#;
        assert!(!parse_tool_calls_fallible(t).is_empty());
    }
    #[test] fn minimax_m2_with_param() {
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
        assert_eq!(tool.args.get("path").map(|v| v.as_str().unwrap()), Some("."), "path should be '.'");
    }

    #[test] fn debug_buffer_positions() {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        let block = "<invoke name=\"list_dir\">\n<parameter name=\"path\">.</parameter>\n</invoke>\n";
        let mut reader = Reader::from_str(block);
        reader.config_mut().trim_text(true);
        loop {
            let pos = reader.buffer_position();
            match reader.read_event() {
                Ok(e) => {
                    eprintln!("Event: {:?}, pos={}", e, pos);
                    if matches!(e, Event::Eof) { break; }
                }
                Err(_) => break,
            }
        }
    }
}
