#![warn(clippy::all)]

pub mod actor;
pub mod context7;
pub mod diff;
pub mod emit_approval_sink;
pub mod headless;
pub mod inspector;
pub mod path_utils;
pub mod permission_gate;
pub mod safety;
pub mod stream_response;
pub mod subagent;
pub mod tool_runner;
pub mod truncate;
pub mod turn;

pub use actor::{AgentActor, AgentActorHandle, AgentMsg};
pub use headless::{run_headless_turn, HeadlessOptions, HeadlessResult};
pub use runie_core::tool_parser::{has_tool_calls, parse_tool_calls, ParsedToolCall};
pub use permission_gate::PermissionGate;
pub use runie_core::tool::ToolOutput;
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
#[cfg(test)]
mod truncate_tests;
