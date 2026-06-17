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
            env: std::env::vars().collect(),
            agent_id: None,
            agent_registry: None,
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
