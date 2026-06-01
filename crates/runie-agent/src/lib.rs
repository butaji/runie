#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod config;
pub mod hook;
pub mod state;
pub mod tools;

// Core agent architecture
pub mod events;
pub mod loop_engine;

pub use config::{AgentConfig, ToolExecutionMode};
pub use hook::{Hook, HookDecision, HookError, SafetyHook};
pub use state::AgentState;
pub use loop_engine::{AgentEventStream, AgentLoopConfig};
pub use tools::AgentTool;

// Events re-exports
pub use events::{AgentEvent, AgentMessage, ContentPart, ImageSource, PermissionDecision, TokenUsage, ToolResult};

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests;
