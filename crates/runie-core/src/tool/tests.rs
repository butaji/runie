#![allow(clippy::disallowed_names)]

use crate::tool::resolve_path;
#[cfg(not(feature = "mcp"))]
use crate::tool::ToolStatus;
use crate::tool::{format_bytes, format_duration, format_tool_label, tool_error, tool_status_line};
#[cfg(feature = "mcp")]
use crate::tool::{ToolContext, ToolDef, ToolOutput, ToolStatus};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "mcp")]
use std::time::Duration;

// Test tool definition (requires mcp feature for ToolDef trait)
#[cfg(feature = "mcp")]
struct TestTool;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[allow(dead_code)]
struct TestToolInput {
    input: String,
}

#[cfg(feature = "mcp")]
impl TestTool {
    fn execute_impl(input: TestToolInput) -> ToolOutput {
        ToolOutput {
            tool_name: "test_tool".to_string(),
            tool_args: serde_json::to_value(&input).unwrap_or_default(),
            content: format!("processed: {}", input.input),
            bytes_transferred: None,
            duration: Duration::from_millis(1),
            status: ToolStatus::Success,
        }
    }
}

#[cfg(feature = "mcp")]
impl ToolDef for TestTool {
    type Input = TestToolInput;

    const NAME: &'static str = "test_tool";
    const DESCRIPTION: &'static str = "A test tool";
    const READ_ONLY: bool = false;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, _ctx: &ToolContext) -> ToolOutput {
        Self::execute_impl(input)
    }
}

// Read-only test tool (requires mcp feature for ToolDef trait)
#[cfg(feature = "mcp")]
struct ReadOnlyTestTool;

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ReadOnlyToolInput {}

#[cfg(feature = "mcp")]
impl ToolDef for ReadOnlyTestTool {
    type Input = ReadOnlyToolInput;

    const NAME: &'static str = "read_only_test";
    const DESCRIPTION: &'static str = "A read-only tool";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(_input: Self::Input, _ctx: &ToolContext) -> ToolOutput {
        ToolOutput {
            tool_name: "read_only_test".to_string(),
            tool_args: serde_json::Value::Null,
            content: "read done".to_string(),
            bytes_transferred: None,
            duration: Duration::from_millis(1),
            status: ToolStatus::Success,
        }
    }
}

// ── format helpers ────────────────────────────────────────────────────────

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
    // humantime omits the :00s suffix when seconds == 0.
    assert_eq!(format_duration(60.0), "1m");
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

// ── ToolDef trait tests ─────────────────────────────────────────────────

#[cfg(feature = "mcp")]
#[tokio::test]
async fn tool_def_executes_and_returns_output() {
    let input = TestToolInput { input: "hello".to_string() };
    let ctx = ToolContext::default();
    let output = TestTool::execute(input, &ctx).await;
    assert_eq!(output.tool_name, "test_tool");
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("processed: hello"));
}

#[cfg(feature = "mcp")]
#[test]
#[allow(clippy::assertions_on_constants)]
fn tool_def_constants_are_accessible() {
    assert_eq!(TestTool::NAME, "test_tool");
    assert_eq!(ReadOnlyTestTool::NAME, "read_only_test");
    assert!(!TestTool::READ_ONLY);
    assert!(ReadOnlyTestTool::READ_ONLY);
}

// ── which_tool tests ───────────────────────────────────────────────────────

use crate::tool::format::{which_tool, which_tool_async};

#[test]
fn which_tool_finds_known_executable() {
    // `echo` should be available on all Unix systems
    let result = which_tool("echo");
    assert!(result.is_some(), "should find 'echo' on PATH: {:?}", result);
    let path = result.unwrap();
    assert!(!path.is_empty(), "path should not be empty");
    // Path should end with "echo"
    assert!(
        path.ends_with("echo") || path.contains("echo"),
        "path should contain 'echo': {}",
        path
    );
}

#[test]
fn which_tool_returns_none_for_missing_executable() {
    let result = which_tool("this-executable-does-not-exist-12345");
    assert!(
        result.is_none(),
        "should not find non-existent executable: {:?}",
        result
    );
}

#[tokio::test]
async fn which_tool_async_finds_known_executable() {
    let result = which_tool_async("echo").await;
    assert!(
        result.is_some(),
        "async which should find 'echo' on PATH: {:?}",
        result
    );
}

#[tokio::test]
async fn which_tool_async_returns_none_for_missing_executable() {
    let result = which_tool_async("this-executable-does-not-exist-12345").await;
    assert!(
        result.is_none(),
        "async which should not find non-existent executable: {:?}",
        result
    );
}
