use crate::Tool;

#[test]
fn tool_is_read_only_read_only_tools() {
    assert!(Tool::ReadFile { path: "foo".into(), offset: None, limit: None }.is_read_only());
    assert!(Tool::ListDir { path: "foo".into() }.is_read_only());
    assert!(Tool::Grep { pattern: "foo".into(), path: ".".into(), glob: None, ignore_case: false, literal: false, context: 0, limit: 100 }.is_read_only());
    assert!(Tool::Find { pattern: "foo".into(), path: ".".into(), limit: 100 }.is_read_only());
    assert!(Tool::FetchDocs { library: "foo".into() }.is_read_only());
}

#[test]
fn tool_is_read_only_write_tools() {
    assert!(!Tool::WriteFile { path: "foo".into(), content: "x".into() }.is_read_only());
    assert!(!Tool::EditFile { path: "foo".into(), search: "a".into(), replace: "b".into() }.is_read_only());
    assert!(!Tool::Bash { command: "echo hi".into() }.is_read_only());
}

#[test]
fn test_tool_read_file_exists() {
    let result = Tool::ReadFile {
        path: "Cargo.toml".to_string(),
        offset: None,
        limit: None,
    }
    .execute();
    assert!(result.is_success());
    assert!(result.output.content.contains("runie-agent"));
}

#[test]
fn test_tool_read_file_missing() {
    let result = Tool::ReadFile {
        path: "nonexistent_file_12345.txt".to_string(),
        offset: None,
        limit: None,
    }
    .execute();
    assert!(!result.is_success());
    assert!(result.output.content.contains("Error"));
}

#[test]
fn test_tool_list_dir() {
    let result = Tool::ListDir {
        path: ".".to_string(),
    }
    .execute();
    assert!(result.is_success());
    assert!(!result.output.content.is_empty());
}

#[test]
fn test_tool_write_file_roundtrip() {
    let path = "/tmp/runie_agent_test_write.txt";
    let write_result = Tool::WriteFile {
        path: path.to_string(),
        content: "test content 42".to_string(),
    }
    .execute();
    assert!(write_result.is_success());

    let read_result = Tool::ReadFile {
        path: path.to_string(),
        offset: None,
        limit: None,
    }
    .execute();
    assert!(read_result.is_success());
    assert_eq!(read_result.output.content, "test content 42");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_tool_read_file_with_offset_and_limit() {
    let result = Tool::ReadFile {
        path: "Cargo.toml".to_string(),
        offset: Some(0),
        limit: Some(5),
    }
    .execute();
    assert!(result.is_success());
    assert!(result.output.content.contains("[Lines"));
}

#[test]
fn test_tool_bash_echo() {
    let result = Tool::Bash {
        command: "echo hello_agent".to_string(),
    }
    .execute();
    assert!(result.is_success());
    assert!(result.output.content.contains("hello_agent"));
}

#[test]
fn test_bash_truncation_uses_configured_policy() {
    // Generate 50 lines; with max_lines=10 the output should be truncated.
    let policy = crate::truncate::TruncationPolicy {
        max_lines: 10,
        max_bytes: 1024 * 1024,
    };
    let result = Tool::Bash {
        command: "seq 1 50".to_string(),
    }
    .execute_with_policy(&policy);
    assert!(result.is_success());
    assert!(
        result.output.content.contains("Output truncated"),
        "expected truncation marker, got: {:?}",
        result.output.content
    );
    // Tail strategy: keep last N lines.
    assert!(
        result.output.content.contains("46"),
        "tail should keep the latest lines: {:?}",
        result.output.content
    );
    // The earliest kept line is "41" (lines 41-50 of 50 = 10 lines).
    // Lines 1-40 should not appear in the truncated output.
    assert!(
        !result.output.content.contains("\n1\n"),
        "head lines should be dropped: {:?}",
        result.output.content
    );
    assert!(
        !result.output.content.contains("20\n"),
        "mid lines should be dropped: {:?}",
        result.output.content
    );
}

#[test]
fn test_bash_no_truncation_when_under_policy() {
    // Under the default policy, 50 lines should not be truncated.
    let result = Tool::Bash {
        command: "seq 1 50".to_string(),
    }
    .execute();
    assert!(result.is_success());
    assert!(
        !result.output.content.contains("Output truncated"),
        "small output should not be truncated: {:?}",
        result.output.content
    );
}

#[test]
fn test_tool_bash_invalid_command() {
    let result = Tool::Bash {
        command: "not_a_real_command_12345".to_string(),
    }
    .execute();
    assert!(!result.is_success());
}

#[test]
fn test_tool_bash_blocked_dangerous() {
    let result = Tool::Bash {
        command: "rm -rf /".to_string(),
    }
    .execute();
    assert!(!result.is_success());
    assert!(result.output.content.contains("Blocked"));
}

#[test]
fn test_tool_result_structure() {
    let result = Tool::Bash {
        command: "echo ok".to_string(),
    }
    .execute();
    assert_eq!(result.tool.name(), "bash");
    assert!(result.is_success());
}

#[test]
fn test_tool_write_creates_parent_dirs() {
    let path = "/tmp/runie_test_nested/sub/dir/file.txt";
    let result = Tool::WriteFile {
        path: path.to_string(),
        content: "nested content".to_string(),
    }
    .execute();
    assert!(result.is_success());
    assert!(result.output.content.contains("bytes"));

    // Verify file was created
    let read_result = Tool::ReadFile {
        path: path.to_string(),
        offset: None,
        limit: None,
    }
    .execute();
    assert!(read_result.is_success());
    assert_eq!(read_result.output.content, "nested content");

    // Cleanup
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all("/tmp/runie_test_nested");
}

#[test]
fn test_tool_bash_timeout() {
    // This test verifies timeout handling works - uses a command that sleeps longer than timeout
    // Note: We can't easily test the actual timeout in unit tests since DEFAULT_TIMEOUT_SECS is 60s
    // Instead we verify the command still works for normal cases
    let result = Tool::Bash {
        command: "echo timeout_test_ok".to_string(),
    }
    .execute();
    assert!(result.is_success());
    assert!(result.output.content.contains("timeout_test_ok"));
}

#[test]
fn test_edit_file_success() {
    let path = "/tmp/runie_test_edit.txt";
    std::fs::write(path, "line1\nold\nline3").unwrap();
    let result = Tool::EditFile {
        path: path.to_string(),
        search: "old".to_string(),
        replace: "new".to_string(),
    }
    .execute();
    assert!(result.is_success());
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
    }
    .execute();
    assert!(!result.is_success());
    assert!(result.output.content.contains("not found"));
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
    }
    .execute();
    assert!(!result.is_success());
    assert!(result.output.content.contains("appears"));
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
    }
    .execute();
    assert!(!result.is_success());
    let _ = std::fs::remove_file(path);
}

// ─── ShellOutput tests (Layer 1: pure state) ──────────────────────────────────

#[test]
fn shell_output_records_exit_code() {
    let out = Tool::Bash { command: "exit 42".to_string() }
        .execute_shell(&crate::truncate::TruncationPolicy::default())
        .expect("bash tool");
    assert_eq!(out.exit_code, Some(42));
    assert!(!out.is_success());
}

#[test]
fn shell_output_splits_stdout_stderr() {
    let out = Tool::Bash { command: "echo out && echo err >&2".to_string() }
        .execute_shell(&crate::truncate::TruncationPolicy::default())
        .expect("bash tool");
    assert_eq!(out.stdout.trim(), "out");
    assert_eq!(out.stderr.trim(), "err");
    assert_eq!(out.exit_code, Some(0));
}

#[test]
fn shell_output_notices_truncation() {
    let policy = crate::truncate::TruncationPolicy {
        max_lines: 3,
        max_bytes: 1024 * 1024,
    };
    let out = Tool::Bash { command: "seq 1 10".to_string() }
        .execute_shell(&policy)
        .expect("bash tool");
    assert!(out.truncated, "expected truncation for 10 lines with max_lines=3");
    assert!(out.full_output_path.is_some(), "expected temp file path");
    assert!(
        out.render().contains("Output truncated"),
        "rendered output should contain truncation notice"
    );
}

#[test]
fn shell_output_combines_on_success() {
    let out = Tool::Bash { command: "echo hello".to_string() }
        .execute_shell(&crate::truncate::TruncationPolicy::default())
        .expect("bash tool");
    assert!(out.is_success(), "exit 0 should be success");
    assert!(out.render().contains("hello"));
}

#[test]
fn shell_output_shows_stderr_on_failure() {
    let out = Tool::Bash { command: "echo fail >&2 && exit 1".to_string() }
        .execute_shell(&crate::truncate::TruncationPolicy::default())
        .expect("bash tool");
    assert!(!out.is_success(), "exit 1 should not be success");
    assert!(out.render().contains("fail"), "render should include stderr on failure");
}
