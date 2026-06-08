use crate::Tool;

#[test]
fn test_tool_read_file_exists() {
    let result = Tool::ReadFile { path: "Cargo.toml".to_string() }.execute();
    assert!(result.success);
    assert!(result.output.contains("runie-agent"));
}

#[test]
fn test_tool_read_file_missing() {
    let result = Tool::ReadFile { path: "nonexistent_file_12345.txt".to_string() }.execute();
    assert!(!result.success);
    assert!(result.output.contains("Error"));
}

#[test]
fn test_tool_list_dir() {
    let result = Tool::ListDir { path: ".".to_string() }.execute();
    assert!(result.success);
    assert!(!result.output.is_empty());
}

#[test]
fn test_tool_write_file_roundtrip() {
    let path = "/tmp/runie_agent_test_write.txt";
    let write_result = Tool::WriteFile {
        path: path.to_string(),
        content: "test content 42".to_string(),
    }.execute();
    assert!(write_result.success);

    let read_result = Tool::ReadFile { path: path.to_string() }.execute();
    assert!(read_result.success);
    assert_eq!(read_result.output, "test content 42");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_tool_bash_echo() {
    let result = Tool::Bash { command: "echo hello_agent".to_string() }.execute();
    assert!(result.success);
    assert!(result.output.contains("hello_agent"));
}

#[test]
fn test_tool_bash_invalid_command() {
    let result = Tool::Bash { command: "not_a_real_command_12345".to_string() }.execute();
    assert!(!result.success);
}

#[test]
fn test_tool_bash_blocked_dangerous() {
    let result = Tool::Bash { command: "rm -rf /".to_string() }.execute();
    assert!(!result.success);
    assert!(result.output.contains("Blocked"));
}

#[test]
fn test_tool_result_structure() {
    let result = Tool::Bash { command: "echo ok".to_string() }.execute();
    assert_eq!(result.tool.name(), "bash");
    assert!(result.success);
}

#[test]
fn test_edit_file_success() {
    let path = "/tmp/runie_test_edit.txt";
    std::fs::write(path, "line1\nold\nline3").unwrap();
    let result = Tool::EditFile {
        path: path.to_string(),
        search: "old".to_string(),
        replace: "new".to_string(),
    }.execute();
    assert!(result.success);
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\nnew\nline3");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_edit_file_search_not_found() {
    let path = "/tmp/runie_test_edit2.txt";
    std::fs::write(path, "line1\nline2").unwrap();
    let result = Tool::EditFile {
        path: path.to_string(),
        search: "missing".to_string(),
        replace: "new".to_string(),
    }.execute();
    assert!(!result.success);
    assert!(result.output.contains("not found"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_edit_file_multiple_matches() {
    let path = "/tmp/runie_test_edit3.txt";
    std::fs::write(path, "old\nold\nold").unwrap();
    let result = Tool::EditFile {
        path: path.to_string(),
        search: "old".to_string(),
        replace: "new".to_string(),
    }.execute();
    assert!(!result.success);
    assert!(result.output.contains("appears"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_edit_file_empty_search() {
    let path = "/tmp/runie_test_edit4.txt";
    std::fs::write(path, "content").unwrap();
    let result = Tool::EditFile {
        path: path.to_string(),
        search: "".to_string(),
        replace: "x".to_string(),
    }.execute();
    assert!(!result.success);
    let _ = std::fs::remove_file(path);
}
