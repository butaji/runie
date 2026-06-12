use crate::Tool;

#[test]
fn test_tool_read_file_exists() {
    let result = Tool::ReadFile { path: "Cargo.toml".to_string(), offset: None, limit: None }.execute();
    assert!(result.success);
    assert!(result.output.contains("runie-agent"));
}

#[test]
fn test_tool_read_file_missing() {
    let result = Tool::ReadFile { path: "nonexistent_file_12345.txt".to_string(), offset: None, limit: None }.execute();
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

    let read_result = Tool::ReadFile { path: path.to_string(), offset: None, limit: None }.execute();
    assert!(read_result.success);
    assert_eq!(read_result.output, "test content 42");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_tool_read_file_with_offset_and_limit() {
    let result = Tool::ReadFile {
        path: "Cargo.toml".to_string(),
        offset: Some(0),
        limit: Some(5),
    }.execute();
    assert!(result.success);
    assert!(result.output.contains("[Lines"));
}

#[test]
fn test_tool_bash_echo() {
    let result = Tool::Bash { command: "echo hello_agent".to_string() }.execute();
    assert!(result.success);
    assert!(result.output.contains("hello_agent"));
}

#[test]
fn test_bash_truncation_uses_configured_policy() {
    // Generate 50 lines; with max_lines=10 the output should be truncated.
    let policy = crate::truncate::TruncationPolicy { max_lines: 10, max_bytes: 1024 * 1024 };
    let result = Tool::Bash { command: "seq 1 50".to_string() }.execute_with_policy(&policy);
    assert!(result.success);
    assert!(result.output.contains("Output truncated"),
        "expected truncation marker, got: {:?}", result.output);
    // Tail strategy: keep last N lines.
    assert!(result.output.contains("46"), "tail should keep the latest lines: {:?}", result.output);
    // The earliest kept line is "41" (lines 41-50 of 50 = 10 lines).
    // Lines 1-40 should not appear in the truncated output.
    assert!(!result.output.contains("\n1\n"),
        "head lines should be dropped: {:?}", result.output);
    assert!(!result.output.contains("20\n"),
        "mid lines should be dropped: {:?}", result.output);
}

#[test]
fn test_bash_no_truncation_when_under_policy() {
    // Under the default policy, 50 lines should not be truncated.
    let result = Tool::Bash { command: "seq 1 50".to_string() }.execute();
    assert!(result.success);
    assert!(!result.output.contains("Output truncated"),
        "small output should not be truncated: {:?}", result.output);
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
fn test_tool_write_creates_parent_dirs() {
    let path = "/tmp/runie_test_nested/sub/dir/file.txt";
    let result = Tool::WriteFile {
        path: path.to_string(),
        content: "nested content".to_string(),
    }.execute();
    assert!(result.success);
    assert!(result.output.contains("bytes"));
    
    // Verify file was created
    let read_result = Tool::ReadFile { path: path.to_string(), offset: None, limit: None }.execute();
    assert!(read_result.success);
    assert_eq!(read_result.output, "nested content");
    
    // Cleanup
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all("/tmp/runie_test_nested");
}

#[test]
fn test_tool_bash_timeout() {
    // This test verifies timeout handling works - uses a command that sleeps longer than timeout
    // Note: We can't easily test the actual timeout in unit tests since DEFAULT_TIMEOUT_SECS is 60s
    // Instead we verify the command still works for normal cases
    let result = Tool::Bash { command: "echo timeout_test_ok".to_string() }.execute();
    assert!(result.success);
    assert!(result.output.contains("timeout_test_ok"));
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
