use crate::{
    parser::{has_tool_calls, parse_tool_calls},
    Tool,
};

fn tool_names(tools: &[Tool]) -> Vec<String> {
    tools.iter().map(|t| t.name().to_string()).collect()
}

#[test]
fn test_parse_read_file_tool() {
    let tools = parse_tool_calls("TOOL:read_file:Cargo.toml");
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0],
        Tool::ReadFile {
            path: "Cargo.toml".to_string(),
            offset: None,
            limit: None
        }
    );
}

#[test]
fn test_parse_list_dir_tool() {
    let tools = parse_tool_calls("TOOL:list_dir:src");
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0],
        Tool::ListDir {
            path: "src".to_string()
        }
    );
}

#[test]
fn test_parse_write_file_tool() {
    let tools = parse_tool_calls("TOOL:write_file:hello.txt:Hello World");
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0],
        Tool::WriteFile {
            path: "hello.txt".to_string(),
            content: "Hello World".to_string()
        }
    );
}

#[test]
fn test_parse_bash_tool() {
    let tools = parse_tool_calls("TOOL:bash:echo hello");
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0],
        Tool::Bash {
            command: "echo hello".to_string()
        }
    );
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
    assert_eq!(
        tools[0],
        Tool::WriteFile {
            path: "test.txt".to_string(),
            content: "line1:line2".to_string()
        }
    );
}

#[test]
fn test_parse_structured_edit_tool() {
    let text = r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert!(
        matches!(&tools[0], Tool::EditFile { path, search, replace } if path == "src/main.rs" && search == "old" && replace == "new"
        )
    );
}

#[test]
fn test_parse_structured_bash_tool() {
    let text = r#"{"name": "bash", "arguments": {"command": "echo hello"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert!(
        matches!(&tools[0], Tool::Bash { command } if command == "echo hello"
        )
    );
}

#[test]
fn test_parse_structured_read_file() {
    let text = r#"{"name": "read_file", "arguments": {"path": "Cargo.toml"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert!(
        matches!(&tools[0], Tool::ReadFile { path, .. } if path == "Cargo.toml"
        )
    );
}

#[test]
fn test_parse_mixed_formats() {
    let text =
        "TOOL:bash:echo hi\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"Cargo.toml\"}}";
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
    assert!(matches!(&tools[0], Tool::FetchDocs { library } if library == "tokio"));
}

#[test]
fn parse_tool_calls_legacy_and_json_agree_with_core() {
    let text =
        "TOOL:bash:echo hi\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"Cargo.toml\"}}";
    let typed = parse_tool_calls(text);
    let core = runie_core::tool_markers::parse_tool_calls(text);
    assert_eq!(tool_names(&typed), core);
}

#[test]
fn core_strip_tool_markers_removes_only_tool_lines() {
    let input = "Before\nTOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {}}\nAfter";
    let result = runie_core::tool_markers::strip_tool_markers(input);
    assert_eq!(result, "Before\nAfter");
}
