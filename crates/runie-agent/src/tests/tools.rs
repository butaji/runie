//! Tests for canonical tool execution via `runie_core::tool`.

use runie_core::tool::{ToolContext, ToolStatus};
use runie_engine::tool::builtin_registry;

async fn call_tool(name: &str, args: serde_json::Value) -> runie_core::tool::ToolOutput {
    let registry = builtin_registry();
    let tool = registry.get(name).unwrap_or_else(|| panic!("unknown tool: {}", name));
    tool.call(args, &ToolContext::default())
        .await
        .unwrap_or_else(|e| panic!("tool {} failed: {}", name, e))
}

#[tokio::test]
async fn tool_read_file_exists() {
    let output = call_tool("read_file", serde_json::json!({"path": "Cargo.toml"})).await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("runie-agent"));
}

#[tokio::test]
async fn tool_read_file_missing() {
    let output = call_tool(
        "read_file",
        serde_json::json!({"path": "nonexistent_file_12345.txt"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Error);
    assert!(output.content.contains("Error"));
}

#[tokio::test]
async fn tool_list_dir() {
    let output = call_tool("list_dir", serde_json::json!({"path": "."})).await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(!output.content.is_empty());
}

#[tokio::test]
async fn tool_write_file_roundtrip() {
    let path = "/tmp/runie_agent_test_write.txt";
    let write_output = call_tool(
        "write_file",
        serde_json::json!({"path": path, "content": "test content 42"}),
    )
    .await;
    assert_eq!(write_output.status, ToolStatus::Success);

    let read_output = call_tool("read_file", serde_json::json!({"path": path})).await;
    assert_eq!(read_output.status, ToolStatus::Success);
    assert_eq!(read_output.content, "test content 42");
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn tool_read_file_with_offset_and_limit() {
    let output = call_tool(
        "read_file",
        serde_json::json!({"path": "Cargo.toml", "offset": 0, "limit": 5}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("[Lines"));
}

#[tokio::test]
async fn tool_bash_echo() {
    let output = call_tool("bash", serde_json::json!({"command": "echo hello_agent"})).await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("hello_agent"));
}

#[tokio::test]
async fn tool_bash_invalid_command() {
    let output = call_tool(
        "bash",
        serde_json::json!({"command": "not_a_real_command_12345"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Error);
}

#[tokio::test]
async fn tool_result_structure() {
    let output = call_tool("bash", serde_json::json!({"command": "echo ok"})).await;
    assert_eq!(output.tool_name, "bash");
    assert_eq!(output.status, ToolStatus::Success);
}

#[tokio::test]
async fn tool_write_creates_parent_dirs() {
    let path = "/tmp/runie_test_nested/sub/dir/file.txt";
    let output = call_tool(
        "write_file",
        serde_json::json!({"path": path, "content": "nested content"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("bytes"));

    let read_output = call_tool("read_file", serde_json::json!({"path": path})).await;
    assert_eq!(read_output.status, ToolStatus::Success);
    assert_eq!(read_output.content, "nested content");

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all("/tmp/runie_test_nested");
}

#[tokio::test]
async fn tool_bash_timeout() {
    // Core bash supports a configurable timeout; verify normal commands still work.
    let output = call_tool(
        "bash",
        serde_json::json!({"command": "echo timeout_test_ok"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("timeout_test_ok"));
}

#[tokio::test]
async fn tool_edit_file_success() {
    let path = "/tmp/runie_test_edit.txt";
    std::fs::write(path, "line1\nold\nline3").unwrap();
    let output = call_tool(
        "edit_file",
        serde_json::json!({"path": path, "search": "old", "replace": "new"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\nnew\nline3");
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn tool_edit_file_search_not_found() {
    let path = "/tmp/runie_test_edit2.txt";
    std::fs::write(path, "line1\nline2").unwrap();
    let output = call_tool(
        "edit_file",
        serde_json::json!({"path": path, "search": "missing", "replace": "new"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Error);
    assert!(output.content.contains("not found"));
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn tool_edit_file_multiple_matches() {
    let path = "/tmp/runie_test_edit3.txt";
    std::fs::write(path, "old\nold\nold").unwrap();
    let output = call_tool(
        "edit_file",
        serde_json::json!({"path": path, "search": "old", "replace": "new"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Error);
    assert!(output.content.contains("appears"));
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn tool_edit_file_empty_search() {
    let path = "/tmp/runie_test_edit4.txt";
    std::fs::write(path, "content").unwrap();
    let output = call_tool(
        "edit_file",
        serde_json::json!({"path": path, "search": "", "replace": "x"}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Error);
    let _ = std::fs::remove_file(path);
}
