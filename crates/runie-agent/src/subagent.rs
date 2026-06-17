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
use runie_core::event::AgentEvent;
use runie_core::model::ThinkingLevel;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubagentError {
    #[error("provider error: {0}")]
    Provider(String),
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
    provider_key: &str,
    model: &str,
    thinking_level: ThinkingLevel,
    read_only: bool,
    skills_context: &str,
    system_prompt: &str,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let provider = crate::build_provider_with_warning(provider_key, model)
        .map_err(|e| SubagentError::Provider(e.to_string()))?;

    let cmd = build_subagent_command(
        prompt,
        provider_key,
        model,
        thinking_level,
        read_only,
        skills_context,
        system_prompt,
    );

    let rt = build_subagent_runtime()?;
    rt.block_on(run_subagent_turn(&provider, &cmd, max_iterations))
}

fn build_subagent_command(
    prompt: &str,
    provider_key: &str,
    model: &str,
    thinking_level: ThinkingLevel,
    read_only: bool,
    skills_context: &str,
    system_prompt: &str,
) -> AgentCommand {
    AgentCommand {
        content: prompt.to_string(),
        id: "subagent.0".to_string(),
        provider: provider_key.to_string(),
        model: model.to_string(),
        thinking_level,
        read_only,
        skills_context: skills_context.to_string(),
        system_prompt: system_prompt.to_string(),
        truncation: crate::truncate::TruncationPolicy::default(),
    }
}

fn build_subagent_runtime() -> Result<tokio::runtime::Runtime, SubagentError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| SubagentError::Agent(e.to_string()))
}

async fn run_subagent_turn(
    provider: &runie_provider::DynProvider,
    cmd: &AgentCommand,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let state = Arc::new(SubagentState::default());
    let callback = build_subagent_callback(state.clone());

    run_agent_turn(provider, cmd, callback, max_iterations)
        .await
        .map_err(|e| SubagentError::Agent(e.to_string()))?;

    finalize_subagent_result(state)
}

#[derive(Default)]
struct SubagentState {
    responses: Mutex<Vec<String>>,
    done: Mutex<bool>,
    error: Mutex<Option<String>>,
}

fn build_subagent_callback(
    state: Arc<SubagentState>,
) -> Arc<Mutex<dyn FnMut(runie_core::Event) + Send + Sync>> {
    Arc::new(Mutex::new(move |evt: runie_core::Event| match evt {
        AgentEvent::ResponseDelta { content, .. } | AgentEvent::Response { content, .. } => {
            state.responses.lock().unwrap().push(content)
        }
        AgentEvent::Error { message, .. } => *state.error.lock().unwrap() = Some(message),
        AgentEvent::Done { .. } => *state.done.lock().unwrap() = true,
        _ => {}
    }))
}

fn finalize_subagent_result(state: Arc<SubagentState>) -> Result<String, SubagentError> {
    if let Some(msg) = state.error.lock().unwrap().take() {
        return Err(SubagentError::Agent(msg));
    }
    if !*state.done.lock().unwrap() {
        return Err(SubagentError::Agent("subagent did not finish".into()));
    }
    Ok(state.responses.lock().unwrap().join(""))
}
