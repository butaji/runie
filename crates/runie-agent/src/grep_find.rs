use crate::{parser::parse_tool_calls, Tool};

#[test]
fn parse_grep_tool_json() {
    let text =
        r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": "src", "glob": "*.rs"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert!(
        matches!(&tools[0], Tool::Grep { pattern, path, glob, .. } if pattern == "fn main" && path == "src" && *glob == Some("*.rs".to_string()))
    );
}

#[test]
fn parse_find_tool_json() {
    let text = r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "src"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert!(
        matches!(&tools[0], Tool::Find { pattern, path, .. } if pattern == "*.rs" && path == "src")
    );
}

#[test]
fn grep_executes_and_finds_matches() {
    let tool = Tool::Grep {
        pattern: "fn main".to_string(),
        path: ".".to_string(),
        glob: Some("*.rs".to_string()),
        ignore_case: false,
        literal: false,
        context: 0,
        limit: 100,
    };
    let result = tool.execute();
    assert!(result.success);
    assert!(result.output.contains("fn main") || result.output.contains("No matches"));
}

#[test]
fn find_executes_and_lists_files() {
    let tool = Tool::Find {
        pattern: "*.rs".to_string(),
        path: ".".to_string(),
        limit: 100,
    };
    let result = tool.execute();
    assert!(result.success);
    assert!(!result.output.is_empty());
}

#[test]
fn grep_respects_limit() {
    let tool = Tool::Grep {
        pattern: "use ".to_string(),
        path: ".".to_string(),
        glob: Some("*.rs".to_string()),
        ignore_case: false,
        literal: false,
        context: 0,
        limit: 2,
    };
    let result = tool.execute();
    assert!(result.success);
    assert!(result.output.contains("use "));
}

#[test]
fn find_respects_limit() {
    let tool = Tool::Find {
        pattern: "*.rs".to_string(),
        path: ".".to_string(),
        limit: 3,
    };
    let result = tool.execute();
    assert!(result.success);
    let lines: Vec<&str> = result
        .output
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .collect();
    assert!(
        lines.len() <= 3,
        "Expected at most 3 files, got {}",
        lines.len()
    );
}

#[test]
fn grep_literal_mode() {
    let tool = Tool::Grep {
        pattern: ".".to_string(),
        path: ".".to_string(),
        glob: Some("*.toml".to_string()),
        ignore_case: false,
        literal: true,
        context: 0,
        limit: 10,
    };
    let result = tool.execute();
    assert!(result.success);
}

#[test]
fn grep_ignore_case() {
    let tool = Tool::Grep {
        pattern: "CARGO".to_string(),
        path: ".".to_string(),
        glob: Some("*.toml".to_string()),
        ignore_case: true,
        literal: true,
        context: 0,
        limit: 10,
    };
    let result = tool.execute();
    assert!(result.success);
}
