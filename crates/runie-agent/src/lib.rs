#![warn(clippy::all)]

pub mod actor;
pub mod agent_command_builder;
pub mod constants;
pub mod emit_approval_sink;
pub mod headless;
pub mod headless_helper;
pub mod safety;
pub mod stream_response;
pub mod streaming_parser;
pub mod subagent;
pub mod think_filter;
pub mod tool;
pub mod tool_registry;
pub mod tool_runner;
pub mod truncate;
pub mod turn;

pub use actor::leader::{AgentActorFactoryImpl, LeaderAgentHandleImpl};
pub use actor::{spawn_ractor_agent, AgentMsg, RactorAgentHandle};
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
    /// Cancellation token for aborting the provider stream mid-flight.
    /// When cancelled (e.g. via `/new` or `AbortTurn`), the stream stops yielding.
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl Default for AgentCommand {
    fn default() -> Self {
        Self {
            content: String::new(),
            id: String::new(),
            provider: String::new(),
            model: String::new(),
            thinking_level: runie_core::model::ThinkingLevel::Off,
            read_only: false,
            skills_context: String::new(),
            system_prompt: String::new(),
            truncation: crate::truncate::TruncationPolicy::default(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }
}

#[cfg(test)]
mod tests;
