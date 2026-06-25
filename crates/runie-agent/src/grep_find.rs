use runie_core::tool_parser::parse_tool_calls;
use crate::tool::builtin_registry;

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

#[test]
fn grep_and_find_registered() {
    let registry = builtin_registry();
    assert!(registry.get("grep").is_some());
    assert!(registry.get("find").is_some());
}
