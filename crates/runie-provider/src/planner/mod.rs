//! One-shot Orchestrator LLM planner.
//!
//! Calls the planner model once with a structured prompt and parses the response
//! into an `OrchestratorPlan`. Retries on parse failure up to `max_retries` times.

mod config;
mod engine;
mod error;
mod parser;
mod prompt;
mod types;
mod validation;

#[cfg(test)]
mod tests;

pub use config::PlannerConfig;
pub use engine::OneShotPlanner;
pub use error::PlannerError;
pub use types::{PlanInput, ProjectContext, ToolDescription};
pub use validation::validate_plan;
