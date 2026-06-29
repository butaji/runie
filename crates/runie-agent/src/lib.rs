#![warn(clippy::all)]

pub mod actor;
pub mod emit_approval_sink;
pub mod headless;
pub mod headless_helper;
pub mod safety;
pub mod stream_response;
pub mod subagent;
pub mod think_filter;
pub mod tool;
pub mod tool_runner;
pub mod truncate;
pub mod turn;

pub use actor::{RactorAgentHandle, spawn_ractor_agent, RactorAgentHandleExt, AgentMsg};
pub use headless::{
    run_headless_cli, run_headless_turn, HeadlessCliOptions, HeadlessOptions, HeadlessResult,
};
pub use runie_core::permissions::PermissionGate;
pub use runie_core::tool::ToolOutput;
pub use runie_core::tool::{has_tool_calls, parse_tool_calls, ParsedToolCall};
pub use turn::{run_agent_turn, run_agent_turn_with_skills};

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
    pub provider: String,
    pub model: String,
    pub thinking_level: runie_core::model::ThinkingLevel,
    pub read_only: bool,
    pub skills_context: String,
    pub system_prompt: String,
    /// Truncation policy for tool output. Defaults to 2000 lines / 50KB.
    pub truncation: crate::truncate::TruncationPolicy,
}

#[cfg(test)]
mod grep_find;
#[cfg(test)]
mod tests;
