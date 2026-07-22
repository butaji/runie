use runie_core::tool::{has_tool_calls, parse_tool_calls, parse_tool_calls_fallible};

#[test]
fn test_parse_markup_bash_tool() {
    let text = r#"[TOOL_CALL]{tool => "bash", args => {"command" => "echo hi"}}[/TOOL_CALL]"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[0].args["command"], "echo hi");
}

#[test]
fn test_parse_markup_read_file_tool() {
    let text = r#"[TOOL_CALL]{tool => "read_file", args => {"path" => "Cargo.toml"}}[/TOOL_CALL]"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].args["path"], "Cargo.toml");
}

#[test]
fn test_parse_markup_unknown_tool_ignored() {
    let text = r#"[TOOL_CALL]{tool => "unknown_tool", args => {}}[/TOOL_CALL]"#;
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}

#[test]
fn test_parse_markup_malformed_markup_error() {
    let text = r#"[TOOL_CALL]{tool => "bash", args => {}}"#;
    let results = parse_tool_calls_fallible(text);
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
}

#[test]
fn test_parse_markup_mixed_with_other_formats() {
    let text = r#"
TOOL:bash:echo hi
{"name": "read_file", "arguments": {"path": "Cargo.toml"}}
[TOOL_CALL]{tool => "list_dir", args => {"path" => "src"}}[/TOOL_CALL]
"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[1].name, "read_file");
    assert_eq!(tools[2].name, "list_dir");
}

#[test]
fn test_parse_read_file_tool() {
    let tools = parse_tool_calls("TOOL:read_file:Cargo.toml");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].args["path"], "Cargo.toml");
}

#[test]
fn test_parse_list_dir_tool() {
    let tools = parse_tool_calls("TOOL:list_dir:src");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "list_dir");
    assert_eq!(tools[0].args["path"], "src");
}

#[test]
fn test_parse_write_file_tool() {
    let tools = parse_tool_calls("TOOL:write_file:hello.txt:Hello World");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "write_file");
    assert_eq!(tools[0].args["path"], "hello.txt");
    assert_eq!(tools[0].args["content"], "Hello World");
}

#[test]
fn test_parse_bash_tool() {
    let tools = parse_tool_calls("TOOL:bash:echo hello");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[0].args["command"], "echo hello");
}

#[test]
fn test_parse_multiple_tools() {
    let tools = parse_tool_calls("TOOL:read_file:a.txt\nTOOL:read_file:b.txt");
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_parse_no_tools() {
    let tools = parse_tool_calls("Hello, how can I help you?");
    assert!(tools.is_empty());
}

#[test]
fn test_parse_unknown_tool_ignored() {
    let tools = parse_tool_calls("TOOL:unknown_tool:arg");
    assert!(tools.is_empty());
}

#[test]
fn test_has_tool_calls_true() {
    assert!(has_tool_calls("TOOL:bash:ls"));
}

#[test]
fn test_has_tool_calls_false() {
    assert!(!has_tool_calls("Just a plain response"));
}

#[test]
fn test_parse_tool_with_extra_colons_in_content() {
    let tools = parse_tool_calls("TOOL:write_file:test.txt:line1:line2");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "write_file");
    assert_eq!(tools[0].args["path"], "test.txt");
    assert_eq!(tools[0].args["content"], "line1:line2");
}

#[test]
fn test_parse_structured_edit_tool() {
    let text = r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "edit_file");
    assert_eq!(tools[0].args["path"], "src/main.rs");
    assert_eq!(tools[0].args["search"], "old");
    assert_eq!(tools[0].args["replace"], "new");
}

#[test]
fn test_parse_structured_bash_tool() {
    let text = r#"{"name": "bash", "arguments": {"command": "echo hello"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert_eq!(tools[0].args["command"], "echo hello");
}

#[test]
fn test_parse_structured_read_file() {
    let text = r#"{"name": "read_file", "arguments": {"path": "Cargo.toml"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].args["path"], "Cargo.toml");
}

#[test]
fn test_parse_mixed_formats() {
    let text = "TOOL:bash:echo hi\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"Cargo.toml\"}}";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_parse_invalid_json_ignored() {
    let tools = parse_tool_calls("{\"name\": \"edit_file\", \"arguments\": {broken");
    assert!(tools.is_empty());
}

#[test]
fn test_parse_unknown_structured_tool_ignored() {
    let tools = parse_tool_calls(r#"{"name": "magic", "arguments": {}}"#);
    assert!(tools.is_empty());
}

#[test]
fn test_parse_structured_fetch_docs() {
    let text = r#"{"name": "fetch_docs", "arguments": {"library": "tokio"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "fetch_docs");
    assert_eq!(tools[0].args["library"], "tokio");
}

#[test]
fn parse_tool_calls_returns_names_and_args() {
    let text = r#"{"name": "bash", "arguments": {"command": "ls"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "bash");
    assert!(tools[0].args.is_object());
}

#[test]
fn core_strip_tool_markers_removes_only_tool_lines() {
    let input = "Before\nTOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {}}\nAfter";
    let result = runie_core::tool_markers::strip_tool_markers(input);
    assert_eq!(result, "Before\nAfter");
}

#[test]
fn parse_legacy_bash_tool() {
    let result = parse_tool_calls("TOOL:bash:ls -la");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "bash");
    assert_eq!(result[0].args["command"], "ls -la");
}

#[test]
fn parse_legacy_read_file() {
    let result = parse_tool_calls("TOOL:read_file:src/main.rs");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "read_file");
    assert_eq!(result[0].args["path"], "src/main.rs");
}

#[test]
fn parse_json_tool_call() {
    let json = r#"{"name": "bash", "arguments": {"command": "echo hi"}}"#;
    let result = parse_tool_calls(json);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "bash");
    assert_eq!(result[0].args["command"], "echo hi");
}

#[test]
fn parse_multiple_tool_calls() {
    let text = r#"
TOOL:bash:ls
{"name": "read_file", "arguments": {"path": "Cargo.toml"}}
"#;
    let result = parse_tool_calls(text);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "bash");
    assert_eq!(result[1].name, "read_file");
}

#[test]
fn parse_fallible_returns_errors_for_malformed_calls() {
    let text = r#"
TOOL:unknown_tool:arg
{"name": "bash", "arguments": {"command": "echo hi"}}
{"name": "bash" "arguments": {}}
"#;
    let results = parse_tool_calls_fallible(text);
    assert_eq!(results.len(), 3);
    assert!(results[0].is_err(), "unknown legacy tool should error");
    assert!(results[1].is_ok(), "valid JSON tool should parse");
    assert!(results[2].is_err(), "malformed JSON should error");
}

#[test]
fn parse_error_message_includes_raw_input() {
    use runie_core::message::Role;
    use runie_core::tool::{tool_parse_error_message, ToolParseError};
    let error = ToolParseError { raw: "{bad json".into(), reason: "invalid JSON".into() };
    let msg = tool_parse_error_message(&error, "parse_0");
    assert_eq!(msg.role, Role::Tool);
    assert!(msg.content().contains("{bad json"));
    assert_eq!(msg.tool_call_id, Some("parse_0".into()));
}

#[test]
fn parse_minimax_list_dir_tool() {
    let text = r#"<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "list_dir");
    assert_eq!(tools[0].args["path"], ".");
}

#[test]
fn parse_minimax_multiple_tools() {
    let text = r#"<minimax:tool_call>
<invoke name="read_file">
<parameter name="path">Cargo.toml</parameter>
</invoke>
<invoke name="bash">
<parameter name="command">echo hi</parameter>
</invoke>
</minimax:tool_call>"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[1].name, "bash");
}

#[test]
fn parse_minimax_unknown_tool_ignored() {
    let text = r#"<minimax:tool_call>
<invoke name="unknown_tool">
<parameter name="x">1</parameter>
</invoke>
</minimax:tool_call>"#;
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}

#[test]
fn parse_minimax_mixed_with_text() {
    let text = r#"I'll list the files.
<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>
Done."#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "list_dir");
}

#[test]
fn parse_grep_tool_json() {
    let text = r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": "src", "glob": "*.rs"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "grep");
    assert_eq!(tools[0].args["pattern"], "fn main");
    assert_eq!(tools[0].args["path"], "src");
    assert_eq!(tools[0].args["glob"], "*.rs");
}

#[test]
fn parse_find_tool_json() {
    let text = r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "src"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "find");
    assert_eq!(tools[0].args["pattern"], "*.rs");
    assert_eq!(tools[0].args["path"], "src");
}
