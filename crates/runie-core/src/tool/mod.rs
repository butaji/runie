//! Tool registry and built-in tools for runie-agent.
//!
//! Each tool implements the [`Tool`] trait. The [`builtin_registry`] function creates
//! a [`ToolRegistry`] with all built-ins. This module also provides pure formatting
//! helpers for inline tool rendering.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

// ─── Tool Trait & Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: std::env::vars().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Success,
    Error,
    TimedOut,
    Blocked,
    /// Tool is waiting for user input before it can complete.
    AwaitingUser,
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Name of the tool that was executed.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Rendered output content.
    pub content: String,
    pub bytes_transferred: Option<u64>,
    pub duration: Duration,
    pub status: ToolStatus,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn is_read_only(&self) -> bool {
        false
    }
    fn requires_approval(&self, _input: &Value) -> bool {
        true
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
}

pub struct ToolRegistry {
    pub(crate) tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn list(&self) -> Vec<&Arc<dyn Tool>> {
        self.tools.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.input_schema(),
                })
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Built-in Tools ───────────────────────────────────────────────────────────

mod ask_user;
mod bash;
mod list_dir;
mod read_file;
mod write_file;
mod edit_file;
mod grep;
mod find;
mod fetch_docs;
pub mod search;
pub mod find_definitions;

pub use ask_user::AskUserTool;
pub use bash::BashTool;
pub use list_dir::ListDirTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
pub use edit_file::EditFileTool;
pub use grep::GrepTool;
pub use find::FindTool;
pub use fetch_docs::FetchDocsTool;
pub use search::SearchTool;
pub use find_definitions::FindDefinitionsTool;

/// Create a registry with all built-in tools registered.
pub fn builtin_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(AskUserTool));
    registry.register(Arc::new(BashTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(EditFileTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(GrepTool));
    registry.register(Arc::new(FindTool));
    registry.register(Arc::new(FetchDocsTool));
    registry.register(Arc::new(SearchTool));
    registry.register(Arc::new(FindDefinitionsTool));
    registry
}

// ─── Inline Tool Rendering Helpers ─────────────────────────────────────────────

/// Maximum display width for tool arguments before truncation.
const ARGS_TRUNCATE_WIDTH: usize = 40;

/// Truncate args to a maximum display width, appending '…' if truncated.
fn truncate_args(args: &str) -> String {
    let display_width = crate::display_width::width(args) as usize;
    if display_width <= ARGS_TRUNCATE_WIDTH {
        return args.to_string();
    }

    let mut result = String::new();
    let mut current_width = 0usize;
    for c in args.chars() {
        let char_width = crate::display_width::width(&c.to_string()) as usize;
        if current_width + char_width > ARGS_TRUNCATE_WIDTH - 1 {
            result.push('…');
            break;
        }
        result.push(c);
        current_width += char_width;
    }
    result
}

/// Format a tool label with args, truncated if needed.
///
/// Examples:
/// - `format_tool_label("bash", "echo hi")` → `"Run bash 'echo hi'"`
/// - `format_tool_label("ls", "")` → `"Run ls"`
/// - `format_tool_label("bash", "a very long command...")` → `"Run bash 'a very long comma…'"`
pub fn format_tool_label(name: &str, args: &str) -> String {
    let args = truncate_args(args);
    if args.is_empty() {
        format!("Run {}", name)
    } else {
        format!("Run {} '{}'", name, args)
    }
}

/// Format bytes into human-readable form.
///
/// Examples:
/// - `format_bytes(567)` → `"567"`
/// - `format_bytes(1234)` → `"1.2k"`
/// - `format_bytes(3_456_789)` → `"3.5M"`
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1000 {
        bytes.to_string()
    } else if bytes < 1_000_000 {
        format!("{:.1}k", bytes as f64 / 1000.0)
    } else {
        format!("{:.1}M", bytes as f64 / 1_000_000.0)
    }
}

/// Format duration in seconds.
///
/// Examples:
/// - `format_duration(12.3)` → `"12.3s"`
/// - `format_duration(65.0)` → `"1m5s"`
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let minutes = (secs / 60.0) as u64;
        let remaining = secs - (minutes as f64 * 60.0);
        format!("{}m{:.0}s", minutes, remaining)
    }
}

pub mod state;
pub use state::{ToolCallState, ToolCallTracker};

/// Build the inline status line for a tool block.
///
/// Used by rendering tests to verify the header line format.
///
/// Examples:
/// - Running: `"⠋ Run ls . 1.8s"`
/// - Done with bytes: `"✓ Run ls . 5.7s ⇣21.2k"`
/// - Done with error: `"✗ Run bash 0.5s [✗]"`
pub fn tool_status_line(
    label: &str,
    duration_secs: f64,
    bytes: Option<u64>,
    status: &str,
) -> String {
    let dur = format_duration(duration_secs);
    let bytes_str = bytes.map(|b| format!(" ⇣{}", format_bytes(b))).unwrap_or_default();
    let error_suffix = if status == "✗" { " [✗]" } else { "" };
    format!("{}{} {}{}{}", status, label, dur, bytes_str, error_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(line.contains("1.8s"), "running line should show duration: {}", line);
    }

    #[test]
    fn tool_status_line_done_shows_checkmark() {
        let line = tool_status_line("Run ls", 5.7, Some(21_200), "✓");
        assert!(line.starts_with("✓"), "done status should start with checkmark: {}", line);
        assert!(line.contains("5.7s"), "done line should show duration: {}", line);
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
}
