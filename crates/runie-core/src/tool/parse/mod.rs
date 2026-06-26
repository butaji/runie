//! Text-based fallback parser for tool calls.
//!
//! This module provides parsing for tool calls embedded in plain-text LLM output.
//! It handles inline JSON, MiniMax XML, legacy `TOOL:` format, and `[TOOL_CALL]`
//! markup. This is a fallback for providers that don't emit structured
//! `ProviderEvent::ToolCall*` events.

mod legacy;
mod json;
mod markup;
mod minimax;

pub use legacy::{
    build_legacy_args, parse_legacy_tool, parse_legacy_tools_in_line,
};
pub use json::{parse_inline_json_tools, parse_json_object_at};
pub use markup::{arrow_to_json, extract_tool_call_payload, parse_markup_tool};
pub use minimax::{
    is_known_tool, parse_minimax_invokes, parse_minimax_parameters, parse_minimax_tool_calls,
};

pub use super::types::{ParsedToolCall, ToolParseError};

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
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        results.extend(parse_line_strategies(trimmed, line));
    }
    results
}

/// Parse a trimmed line with all strategies.
fn parse_line_strategies(
    trimmed: &str,
    original_line: &str,
) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    let mut results = Vec::new();
    results.extend(parse_legacy_tools_in_line(trimmed));
    if trimmed.contains('{') {
        let inline = parse_inline_json_tools(trimmed);
        if !inline.is_empty() {
            results.extend(inline);
        } else if trimmed.starts_with('{') {
            results.push(Err(ToolParseError {
                raw: original_line.to_owned(),
                reason: "invalid JSON tool call or unknown tool name".into(),
            }));
        }
    }
    if trimmed.contains("[TOOL_CALL]") {
        match parse_markup_tool(trimmed) {
            Some(t) => results.push(Ok(t)),
            None => results.push(Err(ToolParseError {
                raw: original_line.to_owned(),
                reason: "invalid [TOOL_CALL] markup or unknown tool name".into(),
            })),
        }
    }
    results
}

/// Check if text contains any tool call markers.
pub fn has_tool_calls(text: &str) -> bool {
    if text.contains("[TOOL_CALL]") && text.contains("[/TOOL_CALL]") {
        return true;
    }
    if text.contains(minimax::OPEN_M2) || text.contains(minimax::OPEN_M3) {
        return true;
    }
    text.lines().any(has_tool_calls_in_line)
}

fn has_tool_calls_in_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with("TOOL:") {
        return true;
    }
    if trimmed.starts_with('{') && json::is_tool_call_value_check(trimmed) {
        return true;
    }
    line.match_indices("TOOL:")
        .any(|(idx, _)| parse_legacy_tool(&line[idx + 5..]).is_some())
}

/// Check if a JSON value is a tool call.
pub fn is_tool_call_value(value: &serde_json::Value) -> bool {
    value.get("name").is_some()
        && value.get("arguments").is_some()
        && value
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(minimax::is_known_tool)
}

/// Assign synthetic ids to parsed tool calls.
pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for tool in tools.iter_mut() {
        if tool.id.is_none() {
            tool.id = Some(format!("call_{}", 0));
        }
    }
}

/// Build an assistant message from parsed tool calls.
pub fn build_assistant_message(
    response_text: &str,
    reasoning: Option<&str>,
    tools: &[ParsedToolCall],
) -> crate::message::ChatMessage {
    use crate::message::{ChatMessage, Part, ToolCall};
    let tool_calls: Vec<ToolCall> = tools
        .iter()
        .map(|t| ToolCall::new(t.id.clone().unwrap_or_default(), &t.name, t.args.clone()))
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
    ChatMessage::assistant(response_text.to_owned())
        .with_tool_calls(tool_calls)
        .with_parts(parts)
}

/// Build a tool-result message for a parse error.
pub fn tool_parse_error_message(error: &ToolParseError, id: &str) -> crate::message::ChatMessage {
    crate::message::ChatMessage::tool_result(format!(
        "Could not parse tool call: {}. Raw input: {}",
        error.reason, error.raw
    ))
    .with_tool_call_id(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimax_first() {
        let text = "<minimax:tool_call><invoke name=\"bash\"><parameter name=\"command\">ls</parameter>\n</invoke>\n</minimax:tool_call>";
        let results = parse_tool_calls_fallible(text);
        assert!(!results.is_empty());
        let first = results[0].as_ref().unwrap();
        assert_eq!(first.name, "bash");
    }

    #[test]
    fn parse_legacy_colon_form() {
        let results = parse_tool_calls_fallible("TOOL:bash:ls");
        assert!(!results.is_empty());
        let first = results[0].as_ref().unwrap();
        assert_eq!(first.name, "bash");
    }

    #[test]
    fn parse_legacy_space_form() {
        let results = parse_tool_calls_fallible("TOOL:bash ls");
        assert!(!results.is_empty());
        let first = results[0].as_ref().unwrap();
        assert_eq!(first.name, "bash");
    }

    #[test]
    fn parse_inline_json() {
        let text = r#"{"name":"bash","arguments":{"command":"ls"}}"#;
        let results = parse_tool_calls_fallible(text);
        assert!(!results.is_empty());
        let first = results[0].as_ref().unwrap();
        assert_eq!(first.name, "bash");
    }

    #[test]
    fn parse_markup() {
        let text = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#;
        let results = parse_tool_calls_fallible(text);
        assert!(!results.is_empty());
        let first = results[0].as_ref().unwrap();
        assert_eq!(first.name, "bash");
        assert_eq!(first.args["command"], "ls");
    }

    #[test]
    fn assign_ids() {
        let mut tools = vec![
            ParsedToolCall {
                name: "bash".into(),
                args: serde_json::json!({}),
                id: None,
            },
            ParsedToolCall {
                name: "read".into(),
                args: serde_json::json!({}),
                id: Some("call_0".into()),
            },
        ];
        assign_tool_call_ids(&mut tools);
        assert_eq!(tools[0].id, Some("call_0".into()));
        assert_eq!(tools[1].id, Some("call_0".into()));
    }
}
