//! Single source of truth for parsing tool-call markers from LLM text output.

pub mod inline_json;
pub mod legacy;
pub mod markup;
pub mod minimax;
pub mod repair;

use crate::message::{ChatMessage, Part, ToolCall};
use serde_json::Value;

use self::inline_json::parse_inline_json_tools;
use self::legacy::{parse_legacy_tool, parse_legacy_tools_in_line};
use self::markup::has_tool_call_markup;
use self::markup::parse_tool_call_markup as parse_markup_tool;
use self::minimax::{has_minimax_tool_calls, parse_minimax_tool_calls};

pub use self::repair::repair_partial_json;

/// A parsed tool invocation: name, JSON arguments, and an optional call id.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: Value,
    pub id: Option<String>,
}

/// A tool-call parse error: the raw line and a human-readable reason.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolParseError {
    pub raw: String,
    pub reason: String,
}

/// Parse tool calls from LLM text output.
pub fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    parse_tool_calls_fallible(text)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

/// Parse tool calls, returning both successes and errors.
pub fn parse_tool_calls_fallible(text: &str) -> Vec<Result<ParsedToolCall, ToolParseError>> {
    // Try MiniMax format first
    let minimax = parse_minimax_tool_calls(text);
    if !minimax.is_empty() {
        return minimax;
    }

    // Parse each line with all strategies
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

    // Strategy 1: Legacy TOOL: format
    results.extend(parse_legacy_tools_in_line(trimmed));

    // Strategy 2: Inline JSON
    if trimmed.contains('{') {
        let inline = parse_inline_json_tools(trimmed);
        if !inline.is_empty() {
            results.extend(inline);
        } else if trimmed.starts_with('{') {
            results.push(Err(ToolParseError {
                raw: original_line.to_string(),
                reason: "invalid JSON tool call or unknown tool name".into(),
            }));
        }
    }

    // Strategy 3: [TOOL_CALL] markup
    if has_tool_call_markup(trimmed) {
        match parse_markup_tool(trimmed) {
            Some(t) => results.push(Ok(t)),
            None => results.push(Err(ToolParseError {
                raw: original_line.to_string(),
                reason: "invalid [TOOL_CALL] markup or unknown tool name".into(),
            })),
        }
    }

    results
}

/// Build a tool-result message for a parse error.
pub fn tool_parse_error_message(error: &ToolParseError, id: &str) -> ChatMessage {
    ChatMessage::tool_result(format!(
        "Could not parse tool call: {}. Raw input: {}",
        error.reason, error.raw
    ))
    .with_tool_call_id(id)
}

/// Check if text contains tool call markers.
pub fn has_tool_calls(text: &str) -> bool {
    if text.contains("[TOOL_CALL]") && text.contains("[/TOOL_CALL]") {
        return true;
    }
    if has_minimax_tool_calls(text) {
        return true;
    }
    text.lines().any(has_tool_calls_in_line)
}

/// Check if a single line contains tool call markers.
fn has_tool_calls_in_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with("TOOL:") {
        return true;
    }
    if trimmed.starts_with('{')
        && serde_json::from_str::<Value>(trimmed)
            .ok()
            .is_some_and(|v| is_tool_call_value(&v))
    {
        return true;
    }
    // Check for inline legacy marker
    line.match_indices("TOOL:")
        .any(|(idx, _)| parse_legacy_tool(&line[idx + 5..]).is_some())
}

/// Check if a JSON value is a tool call.
pub fn is_tool_call_value(value: &Value) -> bool {
    value.get("name").is_some()
        && value.get("arguments").is_some()
        && value
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(minimax::is_known_tool)
}

/// Assign synthetic ids to parsed tool calls.
pub fn assign_tool_call_ids(tools: &mut [ParsedToolCall]) {
    for (i, tool) in tools.iter_mut().enumerate() {
        if tool.id.is_none() {
            tool.id = Some(format!("call_{}", i));
        }
    }
}

/// Build an assistant message from parsed tool calls.
pub fn build_assistant_message(
    response_text: &str,
    reasoning: Option<&str>,
    tools: &[ParsedToolCall],
) -> ChatMessage {
    let tool_calls: Vec<ToolCall> = tools
        .iter()
        .map(|t| {
            ToolCall::new(t.id.clone().unwrap_or_default(), &t.name, t.args.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimax_first() {
        // MiniMax format should be tried first
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
    fn has_tool_calls_true_for_legacy() {
        assert!(has_tool_calls("TOOL:bash:ls"));
    }

    #[test]
    fn has_tool_calls_true_for_json() {
        let text = r#"{"name":"bash","arguments":{"command":"ls"}}"#;
        assert!(has_tool_calls(text));
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
        assert_eq!(tools[1].id, Some("call_0".into())); // Preserved existing
    }
}
