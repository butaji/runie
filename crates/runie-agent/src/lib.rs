pub mod config;
pub mod hook;
pub mod state;
pub mod executor;
pub mod loop_mod;
pub mod agent;

// Pi-style agent architecture
pub mod events;
pub mod pi;
pub mod loop_engine;
pub mod harness;

pub use config::{AgentConfig, ToolExecutionMode};
pub use hook::{Hook, HookDecision, HookError, SafetyHook};
pub use state::AgentState;
pub use executor::ToolExecutor;
pub use loop_mod::{AgentLoop, AgentLoopError};
pub use agent::{Agent, AgentError, CodingAgent};

// Pi types re-exports
pub use events::{AgentEvent, AgentMessage, ContentPart, ImageSource, PermissionDecision, TokenUsage, ToolResult};
pub use pi::{AgentState as PiAgentState, AgentTool, Agent as PiAgent, EventListener};

#[cfg(test)]
mod tests;
