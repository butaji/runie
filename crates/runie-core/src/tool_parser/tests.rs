use crate::tool_parser::{
    assign_tool_call_ids, build_assistant_message, has_tool_calls, parse_tool_calls,
    parse_tool_calls_fallible, ParsedToolCall,
};
use serde_json::Value;

#[test]
fn test_parse_markup_bash_tool() {
    let text = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "echo hi"}}[/TOOL_CALL]"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[0].args["command"], "echo hi");
}

#[test]
fn test_parse_markup_unknown_tool_ignored() {
    let text = r#"[TOOL_CALL]{tool => "unknown_tool", args => {}}[/TOOL_CALL]"#;
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}

#[test]
fn test_parse_inline_json_tool() {
    let text = r#"{"name": "bash", "arguments": {"command": "ls"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[0].args["command"], "ls");
}

#[test]
fn test_parse_legacy_tool() {
    let tools = parse_tool_calls("TOOL:read_file:Cargo.toml");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].args["path"], "Cargo.toml");
}

#[test]
fn test_has_tool_calls() {
    assert!(has_tool_calls("TOOL:bash:ls"));
    assert!(!has_tool_calls("Hello world"));
}

#[test]
fn test_assign_tool_call_ids() {
    let mut tools = vec![ParsedToolCall {
        name: "bash".into(),
        args: Value::Null,
        id: None,
    }];
    assign_tool_call_ids(&mut tools);
    assert_eq!(tools[0].id, Some("call_0".into()));
}

#[test]
fn test_parse_tool_calls_fallible_returns_errors() {
    let text = "{invalid json}";
    let results = parse_tool_calls_fallible(text);
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
}

#[test]
fn parse_minimax_m3_delimited_read_file() {
    let text = r#"<think>
Now let me read the README.md file.
</think>

]<]minimax[>[<tool_call>
]<]minimax[>[<invoke name="read_file">]<]minimax[>[<path>README.md]<]minimax[>[</path>]<]minimax[>[</invoke>
]<]minimax[>[</tool_call>"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].args["path"], "README.md");
}

#[test]
fn parse_minimax_m2_parameter_tags() {
    let text = r#"<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].args["path"], ".");
}

#[test]
fn test_build_assistant_message_includes_tools() {
    let tools = vec![ParsedToolCall {
        name: "bash".into(),
        args: Value::Object(
            [("command".into(), Value::String("ls".into()))]
                .into_iter()
                .collect(),
        ),
        id: Some("call_0".into()),
    }];
    let msg = build_assistant_message("hello", None, &tools);
    assert_eq!(msg.role, crate::message::Role::Assistant);
    assert_eq!(msg.tool_calls().len(), 1);
}

#[test]
fn parse_tool_calls_extracts_inline_legacy_marker() {
    let text = "I'll list files.TOOL:list_dir:.";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "list_dir");
    assert_eq!(tools[0].args["path"], ".");
}

#[test]
fn parse_tool_calls_ignores_inline_tool_mention() {
    let text = "Use the TOOL: parameter to configure the tool.";
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}
