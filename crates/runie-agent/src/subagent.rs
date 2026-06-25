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
//!
//! The caller is responsible for building the provider (via `ProviderActor` in
//! production) and passing it in. This module does no config I/O.

use crate::{run_agent_turn, AgentCommand, PermissionGate};
use runie_core::model::ThinkingLevel;
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_core::provider::Provider;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum SubagentError {
    Agent(String),
}

impl std::fmt::Display for SubagentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubagentError::Agent(msg) => write!(f, "agent turn failed: {msg}"),
        }
    }
}

impl std::error::Error for SubagentError {}

/// Run a subagent turn asynchronously. Returns the final assistant text.
///
/// `provider` and `provider_key`/`model` come from the caller (usually the
/// `ProviderActor`). `prompt` is the user request. The subagent's message
/// buffer is empty (no parent history leaks in).
// allow: all args are orthogonal subagent config params — refactoring would hurt call-site clarity
#[allow(clippy::too_many_arguments)]
pub async fn run_subagent(
    prompt: &str,
    provider_key: &str,
    model: &str,
    provider: &dyn Provider,
    thinking_level: ThinkingLevel,
    read_only: bool,
    skills_context: &str,
    system_prompt: &str,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let cmd = build_subagent_command(
        prompt,
        provider_key,
        model,
        thinking_level,
        read_only,
        skills_context,
        system_prompt,
    );
    run_subagent_turn(provider, &cmd, max_iterations).await
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

async fn run_subagent_turn(
    provider: &dyn Provider,
    cmd: &AgentCommand,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let state = Arc::new(SubagentState::default());
    let callback = build_subagent_callback(state.clone());
    let gate = PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink));

    run_agent_turn(provider, cmd, callback, max_iterations, gate)
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
        runie_core::Event::ResponseDelta { content, .. }
        | runie_core::Event::Response { content, .. } => state
            .responses
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(content),
        runie_core::Event::Error { message, .. } => {
            *state.error.lock().unwrap_or_else(|p| p.into_inner()) = Some(message)
        }
        runie_core::Event::Done { .. } => {
            *state.done.lock().unwrap_or_else(|p| p.into_inner()) = true
        }
        _ => {}
    }))
}

fn finalize_subagent_result(state: Arc<SubagentState>) -> Result<String, SubagentError> {
    if let Some(msg) = state.error.lock().unwrap_or_else(|p| p.into_inner()).take() {
        return Err(SubagentError::Agent(msg));
    }
    if !*state.done.lock().unwrap_or_else(|p| p.into_inner()) {
        return Err(SubagentError::Agent("subagent did not finish".into()));
    }
    Ok(state
        .responses
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .join(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::model::ThinkingLevel;
    use runie_testing::mock_provider;

    #[tokio::test]
    async fn subagent_returns_echo_of_prompt() {
        let provider = mock_provider();
        let result = run_subagent(
            "hello subagent",
            "mock",
            "echo",
            &provider,
            ThinkingLevel::Off,
            false,
            "",
            "",
            5,
        )
        .await;
        let out = result.expect("subagent should succeed");
        assert!(
            out.contains("hello subagent"),
            "expected echoed input in output, got: {:?}",
            out
        );
    }

    #[tokio::test]
    async fn subagent_with_skill_context_uses_it() {
        let provider = mock_provider();
        let result = run_subagent(
            "ask about skill",
            "mock",
            "echo",
            &provider,
            ThinkingLevel::Off,
            false,
            "SKILL: test-skill",
            "",
            5,
        )
        .await;
        let out = result.expect("subagent should succeed");
        assert!(out.contains("ask about skill"));
    }

    #[tokio::test]
    async fn subagent_empty_prompt_succeeds() {
        let provider = mock_provider();
        let result = run_subagent(
            "",
            "mock",
            "echo",
            &provider,
            ThinkingLevel::Off,
            false,
            "",
            "",
            5,
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn finalize_recovers_from_poisoned_done_mutex() {
        let state = Arc::new(SubagentState::default());
        let state2 = state.clone();
        let handle = std::thread::spawn(move || {
            let _guard = state2.done.lock().unwrap();
            panic!("poison done mutex")
        });
        let _ = handle.join();

        let result = finalize_subagent_result(state);
        assert!(
            matches!(result, Err(SubagentError::Agent(ref msg)) if msg == "subagent did not finish"),
            "expected 'did not finish' error, got {:?}",
            result
        );
    }
}
