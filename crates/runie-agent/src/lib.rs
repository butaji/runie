#![warn(clippy::all)]

pub mod accumulator;
pub mod diff;
pub mod mutation_queue;
pub mod path_utils;
pub mod parser;
pub mod safety;
pub mod tools;
pub mod truncate;
pub mod turn;

pub use tools::{Tool, ToolResult};
pub use turn::run_agent_turn;

use runie_provider::AnyProvider;

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
    pub provider: String,
    pub model: String,
    pub thinking_level: runie_core::model::ThinkingLevel,
    pub read_only: bool,
}

pub fn build_provider(provider: &str, model: &str) -> AnyProvider {
    AnyProvider::new(provider, model)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod grep_find;
#[cfg(test)]
mod truncate_tests;
