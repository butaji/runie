use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{
    format_bytes, format_duration, format_tool_label, resolve_path, tool_error, tool_status_line,
    Tool, ToolContext, ToolOutput, ToolRegistry, ToolStatus,
};

struct TestTool;

#[async_trait]
impl Tool for TestTool {
    fn name(&self) -> &str {
        "test_tool"
    }
    fn description(&self) -> &str {
        "A test tool"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        })
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput {
            tool_name: "noop".to_string(),
            tool_args: serde_json::Value::Null,
            content: "done".to_string(),
            bytes_transferred: None,
            duration: Duration::from_millis(1),
            status: ToolStatus::Success,
        })
    }
}

struct ReadOnlyTestTool;

#[async_trait]
impl Tool for ReadOnlyTestTool {
    fn name(&self) -> &str {
        "read_only_test"
    }
    fn description(&self) -> &str {
        "A read-only tool"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn is_read_only(&self) -> bool {
        true
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput {
            tool_name: "read_only_test".to_string(),
            tool_args: serde_json::Value::Null,
            content: "read done".to_string(),
            bytes_transferred: None,
            duration: Duration::from_millis(1),
            status: ToolStatus::Success,
        })
    }
}

#[tokio::test]
async fn registry_registers_and_retrieves_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(TestTool));
    let tool = registry.get("test_tool");
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().name(), "test_tool");
}

#[tokio::test]
async fn registry_schemas_include_name_and_description() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(TestTool));
    let schemas = registry.schemas();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0]["name"], "test_tool");
    assert_eq!(schemas[0]["description"], "A test tool");
    assert!(schemas[0]["input_schema"].is_object());
}

#[tokio::test]
async fn read_only_tool_returns_true() {
    let ro = ReadOnlyTestTool;
    assert!(ro.is_read_only());
    let rw = TestTool;
    assert!(!rw.is_read_only());
}

#[tokio::test]
async fn tool_output_records_bytes_and_duration() {
    let tool = TestTool;
    let ctx = ToolContext::default();
    let output = tool
        .call(serde_json::json!({"input": "test"}), &ctx)
        .await
        .unwrap();
    assert!(output.duration.as_millis() >= 1);
    assert_eq!(output.status, ToolStatus::Success);
    assert_eq!(output.content, "done");
}

#[test]
fn format_tool_label_with_args() {
    assert_eq!(format_tool_label("bash", "echo hi"), "Run bash 'echo hi'");
}

#[test]
fn format_tool_label_no_args() {
    assert_eq!(format_tool_label("ls", ""), "Run ls");
}

#[test]
fn format_tool_label_truncates_long_args() {
    // String with display width > 40 to trigger truncation
    let long_args = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 42 chars
    let result = format_tool_label("bash", long_args);
    // Result should contain ellipsis (within the quoted args)
    assert!(result.contains('…'), "result: {}", result);
    assert!(result.starts_with("Run bash '"));
    // The truncated result should be shorter than full args + quotes
    let expected_max_len = "Run bash '".len() + 40 + "…'".len();
    assert!(result.len() <= expected_max_len, "result: {}", result);
}

#[test]
fn format_bytes_small() {
    assert_eq!(format_bytes(0), "0");
    assert_eq!(format_bytes(567), "567");
    assert_eq!(format_bytes(999), "999");
}

#[test]
fn format_bytes_kb() {
    assert_eq!(format_bytes(1000), "1.0k");
    assert_eq!(format_bytes(1_234_567), "1.2M");
}

#[test]
fn format_bytes_mb() {
    assert_eq!(format_bytes(1_000_000), "1.0M");
    assert_eq!(format_bytes(3_456_789), "3.5M");
}

#[test]
fn format_duration_seconds() {
    assert_eq!(format_duration(12.3), "12.3s");
    assert_eq!(format_duration(59.9), "59.9s");
}

#[test]
fn format_duration_minutes() {
    assert_eq!(format_duration(60.0), "1m0s");
    assert_eq!(format_duration(65.0), "1m5s");
    assert_eq!(format_duration(125.0), "2m5s");
}

// ── tool_status_line ────────────────────────────────────────────────────

#[test]
fn tool_status_line_includes_duration() {
    let line = tool_status_line("Run ls", 2.5, None, "✓");
    assert!(
        line.contains("2.5s"),
        "status line should contain duration: {}",
        line
    );
}

#[test]
fn tool_status_line_formats_bytes() {
    let line = tool_status_line("Read file", 1.0, Some(4_930), "✓");
    assert!(
        line.contains("4.9k"),
        "status line should contain formatted bytes: {}",
        line
    );
}

#[test]
fn tool_status_line_running_shows_spinner() {
    let line = tool_status_line("Run ls", 1.8, None, "⠋");
    assert!(
        line.starts_with("⠋"),
        "running status should start with spinner: {}",
        line
    );
    assert!(
        line.contains("1.8s"),
        "running line should show duration: {}",
        line
    );
}

#[test]
fn tool_status_line_done_shows_checkmark() {
    let line = tool_status_line("Run ls", 5.7, Some(21_200), "✓");
    assert!(
        line.starts_with("✓"),
        "done status should start with checkmark: {}",
        line
    );
    assert!(
        line.contains("5.7s"),
        "done line should show duration: {}",
        line
    );
    assert!(line.contains("⇣"), "done line should show bytes: {}", line);
}

#[test]
fn tool_status_line_error_shows_error_icon() {
    let line = tool_status_line("Run bash", 0.5, None, "✗");
    assert!(
        line.starts_with("✗"),
        "error status should start with ✗: {}",
        line
    );
    assert!(line.contains("[✗]"), "error line should show [✗]: {}", line);
}

#[test]
fn resolve_path_absolute_returns_as_is() {
    let abs = std::path::Path::new("/tmp/foo");
    assert_eq!(
        resolve_path("/tmp/foo", std::path::Path::new("/home")),
        abs.to_path_buf()
    );
}

#[test]
fn resolve_path_relative_joins_working_dir() {
    let wd = std::path::Path::new("/home/user");
    assert_eq!(
        resolve_path("src/main.rs", wd),
        std::path::PathBuf::from("/home/user/src/main.rs")
    );
}

#[test]
fn tool_error_returns_error_output() {
    let start = std::time::Instant::now();
    let out = tool_error("bash", "boom", start, false);
    assert_eq!(out.tool_name, "bash");
    assert_eq!(out.content, "boom");
    assert_eq!(out.status, ToolStatus::Error);
}

#[test]
fn tool_error_warning_flag_reports_success() {
    let start = std::time::Instant::now();
    let out = tool_error("grep", "no matches", start, true);
    assert_eq!(out.status, ToolStatus::Success);
}
