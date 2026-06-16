#![warn(clippy::all)]

pub mod accumulator;
pub mod context7;
pub mod diff;
pub mod headless;
pub mod inspector;
pub mod parser;
pub mod path_utils;
pub mod profiles;
pub mod safety;
pub mod subagent;
pub mod tools;
pub mod truncate;
pub mod turn;

pub use headless::{run_headless_turn, HeadlessOptions, HeadlessResult};
pub use parser::{has_tool_calls, parse_tool_calls, ParsedToolCall};
pub use runie_core::tool::ToolOutput;
pub use turn::run_agent_turn;

use runie_provider::DynProvider;

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

/// Build a provider from key and model. Panics on unknown key (callers must validate).
pub fn build_provider(provider: &str, model: &str) -> DynProvider {
    runie_provider::build_provider(provider, model)
}

/// Build a provider, returning an error for unknown or unconfigured providers.
pub fn build_provider_with_warning(
    provider: &str,
    model: &str,
) -> Result<DynProvider, runie_core::ProviderError> {
    runie_provider::build_provider_with_warning(provider, model)
}

#[cfg(test)]
mod grep_find;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod truncate_tests;
