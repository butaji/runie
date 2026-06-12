//! Subagent: a nested agent turn that runs synchronously and returns the
//! final assistant response.
//!
//! Used by `/spawn` in runie-core. The subagent:
//! - Inherits the parent's provider, model, thinking level, read-only flag
//! - Gets a clean message buffer (no parent history)
//! - Runs `run_agent_turn` to completion
//! - Returns the final assistant response as a `String`
//!
//! Errors (network, parse, etc.) are returned as a structured `SubagentError`.

use crate::{run_agent_turn, AgentCommand};
use runie_core::event::Event;
use runie_core::model::ThinkingLevel;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubagentError {
    #[error("agent turn failed: {0}")]
    Agent(String),
}

/// Run a subagent turn synchronously. Returns the final assistant text.
///
/// `prompt` is the user request. The subagent's message buffer is empty
/// (no parent history leaks in), but it uses the same provider, model,
/// and skills context as the parent.
#[allow(clippy::too_many_arguments)]
pub fn run_subagent(
    prompt: &str,
    provider: &str,
    model: &str,
    thinking_level: ThinkingLevel,
    read_only: bool,
    skills_context: &str,
    system_prompt: &str,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let cmd = AgentCommand {
        content: prompt.to_string(),
        id: "subagent.0".to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        thinking_level,
        read_only,
        skills_context: skills_context.to_string(),
        system_prompt: system_prompt.to_string(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };

    // We need a runtime to call the async `run_agent_turn`. We block on
    // a fresh one — subagents are sync from the caller's perspective.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| SubagentError::Agent(e.to_string()))?;

    rt.block_on(async {
        let mut responses: Vec<String> = Vec::new();
        let mut done = false;
        let mut error: Option<String> = None;

        run_agent_turn(
            &cmd,
            |evt| match evt {
                Event::AgentResponse { content, .. } => responses.push(content),
                Event::AgentError { message, .. } => error = Some(message),
                Event::AgentDone { .. } => done = true,
                _ => {}
            },
            max_iterations,
        )
        .await
        .map_err(|e| SubagentError::Agent(e.to_string()))?;

        if let Some(msg) = error {
            return Err(SubagentError::Agent(msg));
        }
        if !done {
            return Err(SubagentError::Agent("subagent did not finish".into()));
        }
        Ok(responses.join(""))
    })
}
