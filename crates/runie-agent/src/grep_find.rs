use runie_core::tool::parse_tool_calls;

#[test]
fn parse_grep_tool_json() {
    let text =
        r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": "src", "glob": "*.rs"}}"#;
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
