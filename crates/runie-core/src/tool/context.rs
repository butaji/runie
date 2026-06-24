//! Tool context and status/output types.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    /// Agent ID when this tool is invoked by a subagent.
    pub agent_id: Option<String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: minimal_tool_env(),
            agent_id: None,
        }
    }
}

/// Returns a minimal, safe environment for tool execution.
///
/// Only well-known, non-sensitive variables needed by typical shell commands
/// are included. Secrets such as API keys, tokens, and passwords are excluded.
fn minimal_tool_env() -> HashMap<String, String> {
    let allowed = [
        "PATH", "HOME", "USER", "SHELL", "TMPDIR", "TMP", "TEMP", "LANG", "LC_ALL", "LC_CTYPE",
        "PWD",
    ];
    let mut env = HashMap::new();
    for key in allowed {
        if let Ok(value) = std::env::var(key) {
            env.insert(key.to_string(), value);
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_excludes_secrets() {
        std::env::set_var("RUNIE_TEST_API_KEY", "should-not-appear");
        let ctx = ToolContext::default();
        assert!(!ctx.env.contains_key("RUNIE_TEST_API_KEY"));
    }

    #[test]
    fn default_context_includes_path() {
        let ctx = ToolContext::default();
        assert!(ctx.env.contains_key("PATH"));
    }

    #[test]
    fn tool_output_success_sets_status_and_duration() {
        let output = ToolOutput::success("test", serde_json::json!({}), "content".into());
        assert_eq!(output.status, ToolStatus::Success);
        assert_eq!(output.content, "content");
        assert_eq!(output.tool_name, "test");
    }

    #[test]
    fn tool_output_success_with_bytes() {
        let output = ToolOutput::success_with_bytes("test", serde_json::json!({}), "content".into(), 100);
        assert_eq!(output.status, ToolStatus::Success);
        assert_eq!(output.bytes_transferred, Some(100));
    }

    #[test]
    fn tool_output_error_sets_error_status() {
        let output = ToolOutput::error("test", serde_json::json!({}), "error msg".into());
        assert_eq!(output.status, ToolStatus::Error);
        assert!(output.content.contains("error msg"));
    }

    #[test]
    fn tool_output_blocked_sets_blocked_status() {
        let output = ToolOutput::blocked("bash", serde_json::json!({}), "unsafe command".into());
        assert_eq!(output.status, ToolStatus::Blocked);
        assert!(output.content.contains("Blocked"));
        assert!(output.content.contains("unsafe command"));
    }

    #[test]
    fn tool_output_json_error_serializes_to_json() {
        let output = ToolOutput::json_error("test", serde_json::json!({}), "something went wrong");
        assert!(output.content.contains("error"));
        assert!(output.content.contains("something went wrong"));
    }

    #[test]
    fn tool_output_json_success_serializes_pretty() {
        let value = serde_json::json!({ "key": "value" });
        let output = ToolOutput::json_success("test", serde_json::json!({}), &value);
        assert!(output.content.contains("key"));
        assert!(output.content.contains("value"));
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

impl ToolOutput {
    /// Create a successful tool output.
    pub fn success(tool_name: &str, tool_args: Value, content: String) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            tool_args,
            content,
            bytes_transferred: None,
            duration: Duration::ZERO,
            status: ToolStatus::Success,
        }
    }

    /// Create a successful tool output with byte count.
    pub fn success_with_bytes(
        tool_name: &str,
        tool_args: Value,
        content: String,
        bytes: u64,
    ) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            tool_args,
            content,
            bytes_transferred: Some(bytes),
            duration: Duration::ZERO,
            status: ToolStatus::Success,
        }
    }

    /// Create an error tool output.
    pub fn error(tool_name: &str, tool_args: Value, content: String) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            tool_args,
            content,
            bytes_transferred: None,
            duration: Duration::ZERO,
            status: ToolStatus::Error,
        }
    }

    /// Create a blocked tool output.
    pub fn blocked(tool_name: &str, tool_args: Value, reason: String) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            tool_args,
            content: format!("Blocked: {}", reason),
            bytes_transferred: None,
            duration: Duration::ZERO,
            status: ToolStatus::Blocked,
        }
    }

    /// Create a JSON success output.
    pub fn json_success(tool_name: &str, tool_args: Value, value: &Value) -> Self {
        Self::success(
            tool_name,
            tool_args,
            serde_json::to_string_pretty(value).unwrap_or_default(),
        )
    }

    /// Create a JSON error output.
    pub fn json_error(tool_name: &str, tool_args: Value, err: &str) -> Self {
        Self::error(
            tool_name,
            tool_args,
            serde_json::to_string(&serde_json::json!({ "error": err })).unwrap_or_default(),
        )
    }
}
