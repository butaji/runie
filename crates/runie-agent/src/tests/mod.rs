//! Tests for runie-agent

use crate::events::{PermissionDecision, ContentPart, AgentEvent, AgentMessage};
use crate::hook::{Hook, HookDecision, SafetyHook};
use crate::loop_engine::AgentLoopConfig;
use crate::state::AgentState;
use futures::StreamExt;
use runie_core::{Message, Session, Context, ToolCall, ToolOutput};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use runie_ai::Provider;
use runie_tools::{create_default_toolkit, Workspace};
use std::path::PathBuf;

mod unit_tests;
mod agent_loop_tests;
mod provider_tests;
mod hook_tests;
mod retry_tests;
mod compaction_tests;
mod tool_id_tests;

pub use unit_tests::*;
pub use agent_loop_tests::*;
pub use provider_tests::*;
pub use hook_tests::*;
pub use retry_tests::*;
pub use compaction_tests::*;
pub use tool_id_tests::*;

/// Creates a test message with the given text content.
pub fn test_message(text: &str) -> AgentMessage {
    AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Creates a default test config.
pub fn test_config(max_turns: usize) -> AgentLoopConfig {
    AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns,
    }
}

/// Creates a test workspace and tool registry.
pub fn test_registry() -> Arc<runie_tools::ToolRegistry> {
    let ws = Workspace::new(PathBuf::from("."));
    Arc::new(create_default_toolkit(ws))
}
