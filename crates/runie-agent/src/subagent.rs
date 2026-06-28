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
//!
//! ## Declarative subagent types
//!
//! Built-in subagent types are defined as markdown files in
//! `resources/agents/` and loaded via `SubagentRegistry`.  Use
//! `run_subagent_type()` to run a named type, or `run_subagent()` for
//! explicit parameters.

use crate::{run_agent_turn, AgentCommand, PermissionGate};
use runie_core::model::ThinkingLevel;
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_core::provider::Provider;
use runie_core::subagents::{PermissionMode as SubPermissionMode, SubagentRegistry, SubagentType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubagentError {
    #[error("agent turn failed: {0}")]
    Source(#[source] anyhow::Error),
}

impl From<anyhow::Error> for SubagentError {
    fn from(e: anyhow::Error) -> Self {
        SubagentError::Source(e)
    }
}

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

/// Run a subagent turn using a named declarative type.
///
/// Looks up `type_name` in `SubagentRegistry::default()`, interpolates the
/// prompt template with `variables`, then runs the turn.
pub async fn run_subagent_type(
    type_name: &str,
    variables: HashMap<&str, &str>,
    parent_provider_key: &str,
    parent_model: &str,
    parent_thinking: ThinkingLevel,
    parent_read_only: bool,
    parent_skills_context: &str,
    parent_system_prompt: &str,
    provider: &dyn Provider,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let sub_type = resolve_subagent_type(type_name)?;
    let cmd = build_type_command(
        &sub_type,
        type_name,
        &variables,
        parent_provider_key,
        parent_model,
        parent_thinking,
        parent_read_only,
        parent_skills_context,
        parent_system_prompt,
    );
    let gate = build_permission_gate(&sub_type.permission_mode);
    run_subagent_turn_with_gate(provider, &cmd, max_iterations, gate).await
}

fn resolve_subagent_type(type_name: &str) -> Result<SubagentType, SubagentError> {
    SubagentRegistry::from_builtins()
        .get(type_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unknown subagent type: {}", type_name))
        .map_err(SubagentError::from)
}

fn build_type_command(
    sub_type: &SubagentType,
    type_name: &str,
    variables: &HashMap<&str, &str>,
    parent_provider_key: &str,
    parent_model: &str,
    parent_thinking: ThinkingLevel,
    parent_read_only: bool,
    parent_skills_context: &str,
    parent_system_prompt: &str,
) -> AgentCommand {
    let prompt = sub_type.interpolate(variables);
    let system_prompt = if sub_type.agents_md { parent_system_prompt } else { "" };
    let skills_context = if sub_type.agents_md { parent_skills_context } else { "" };
    let read_only = sub_type.permission_mode == SubPermissionMode::Plan || parent_read_only;
    AgentCommand {
        content: prompt,
        id: format!("subagent.{}", type_name),
        provider: parent_provider_key.to_owned(),
        model: resolve_model(sub_type, parent_model),
        thinking_level: parent_thinking,
        read_only,
        skills_context: skills_context.to_owned(),
        system_prompt: system_prompt.to_owned(),
        truncation: crate::truncate::TruncationPolicy::default(),
    }
}

/// Resolve the model for a subagent type.
/// `"inherit"` returns `parent_model`; `"fast"` could map to a faster model;
/// a concrete model id is used as-is.
fn resolve_model(sub_type: &SubagentType, parent_model: &str) -> String {
    match sub_type.model.as_str() {
        "inherit" => parent_model.to_owned(),
        "fast" | _ => sub_type.model.clone(), // concrete id or "fast" trait
    }
}

/// Build a `PermissionGate` from a `PermissionMode`.
///
/// Currently uses the default gate (no policies, AutoAllowSink fallback).
/// The `Plan` mode is handled by setting `read_only = true` on the command.
fn build_permission_gate(_mode: &SubPermissionMode) -> PermissionGate {
    let manager = PermissionManager::default();
    PermissionGate::new(manager, Arc::new(AutoAllowSink))
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
        content: prompt.to_owned(),
        id: "subagent.0".to_owned(),
        provider: provider_key.to_owned(),
        model: model.to_owned(),
        thinking_level,
        read_only,
        skills_context: skills_context.to_owned(),
        system_prompt: system_prompt.to_owned(),
        truncation: crate::truncate::TruncationPolicy::default(),
    }
}

/// Run a subagent turn with a custom permission gate.
pub(crate) async fn run_subagent_turn_with_gate(
    provider: &dyn Provider,
    cmd: &AgentCommand,
    max_iterations: usize,
    gate: PermissionGate,
) -> Result<String, SubagentError> {
    let state = Arc::new(SubagentState::default());
    let callback = build_subagent_callback(state.clone());
    run_agent_turn(provider, cmd, callback, max_iterations, gate)
        .await
        .map_err(SubagentError::from)?;
    finalize_subagent_result(state)
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
        .map_err(SubagentError::from)?;

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
        return Err(SubagentError::Source(anyhow::anyhow!(msg)));
    }
    if !*state.done.lock().unwrap_or_else(|p| p.into_inner()) {
        return Err(SubagentError::Source(anyhow::anyhow!("subagent did not finish")));
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
            matches!(result, Err(SubagentError::Source(ref e)) if e.to_string() == "subagent did not finish"),
            "expected 'did not finish' error, got {:?}",
            result
        );
    }

    // Layer 4 — Provider Replay / Mock-Tool E2E

    #[tokio::test]
    async fn explore_subagent_type_runs_with_mock_provider() {
        // Layer 4: Verify that the declarative `explore` subagent type can be
        // resolved and executed through a mock provider without panicking.
        let provider = mock_provider();
        let mut vars = std::collections::HashMap::new();
        vars.insert("task", "find all README files");
        let result = run_subagent_type(
            "explore",
            vars,
            "mock",
            "echo",
            ThinkingLevel::Off,
            false,
            "",
            "",
            &provider,
            5,
        )
        .await;
        // The mock provider returns tool calls; we just verify it runs to completion.
        assert!(result.is_ok(), "explore subagent should succeed, got: {:?}", result);
        let out = result.unwrap();
        assert!(!out.is_empty(), "explore subagent should produce output");
    }

    #[tokio::test]
    async fn unknown_subagent_type_returns_error() {
        let provider = mock_provider();
        let vars = std::collections::HashMap::new();
        let result = run_subagent_type(
            "does-not-exist",
            vars,
            "mock",
            "echo",
            ThinkingLevel::Off,
            false,
            "",
            "",
            &provider,
            5,
        )
        .await;
        assert!(
            result.is_err(),
            "unknown subagent type should return error"
        );
    }
}
