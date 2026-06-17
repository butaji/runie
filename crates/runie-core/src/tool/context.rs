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
    /// Shared subagent registry when running inside Team mode.
    pub agent_registry: Option<std::sync::Arc<std::sync::Mutex<crate::multi_agent::AgentRegistry>>>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: minimal_tool_env(),
            agent_id: None,
            agent_registry: None,
        }
    }
}

/// Returns a minimal, safe environment for tool execution.
///
/// Only well-known, non-sensitive variables needed by typical shell commands
/// are included. Secrets such as API keys, tokens, and passwords are excluded.
fn minimal_tool_env() -> HashMap<String, String> {
    let allowed = [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
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
