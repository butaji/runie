//! Text-based fallback parser for tool calls.
//!
//! Routes through the thin shim in [`super::shim`] which uses `quick-xml`
//! for MiniMax XML parsing and a single-pass JSON detector.

pub use super::shim::{
    assign_tool_call_ids, has_tool_calls, is_known_tool, is_tool_call_value,
    is_tool_call_value_check, parse_inline_json_tools, parse_minimax_tool_calls, parse_tool_calls,
    parse_tool_calls_fallible, OPEN_M2, OPEN_M3,
};
pub use super::types::{ParsedToolCall, ToolParseError};

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
