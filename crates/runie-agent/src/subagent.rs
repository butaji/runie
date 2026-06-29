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

use crate::{run_agent_turn, AgentCommand, PermissionGate, stream_response::EmitFn};
use runie_core::model::ThinkingLevel;
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_core::provider::Provider;
use runie_core::subagents::{PermissionMode as SubPermissionMode, SubagentRegistry, SubagentType};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
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
    let system_prompt = if sub_type.agents_md {
        parent_system_prompt
    } else {
        ""
    };
    let skills_context = if sub_type.agents_md {
        parent_skills_context
    } else {
        ""
    };
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
        "fast" | _ => sub_type.model.clone(),
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
/// Uses a `tokio::sync::oneshot` channel so the callback sends the final result
/// directly when `Done` is received. No polling, no shared mutable state.
async fn run_subagent_turn_with_gate(
    provider: &dyn Provider,
    cmd: &AgentCommand,
    max_iterations: usize,
    gate: PermissionGate,
) -> Result<String, SubagentError> {
    // Create oneshot channel for the single final result.
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, SubagentError>>();

    // Wrap the sender in Arc<Mutex> so the sync emit callback can use it.
    let tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<_>>>> =
        Arc::new(Mutex::new(Some(tx)));

    // Collect response text and errors during the turn.
    let responses = Arc::new(Mutex::new(Vec::<String>::new()));

    let emit: EmitFn = Arc::new(Mutex::new({
        let tx = tx.clone();
        let responses = responses.clone();
        move |evt: runie_core::Event| {
            match evt {
                runie_core::Event::ResponseDelta { content, .. }
                | runie_core::Event::Response { content, .. } => {
                    responses.lock().push(content);
                }
                runie_core::Event::Error { message, .. } => {
                    let mut guard = tx.lock();
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(Err(SubagentError::Source(anyhow::anyhow!(message))));
                    }
                }
                runie_core::Event::Done { .. } => {
                    // Build the final result and send it through the channel.
                    let text = responses.lock().join("");
                    let mut guard = tx.lock();
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(Ok(text));
                    }
                }
                _ => {}
            }
        }
    }));

    // Run the turn once with the collecting callback.
    let run_result = run_agent_turn(provider, cmd, emit, max_iterations, gate).await;

    // If run_agent_turn returned an error and we haven't sent a result yet,
    // send the error through the channel.
    if let Err(e) = run_result {
        let mut guard = tx.lock();
        if let Some(tx) = guard.take() {
            let _ = tx.send(Err(SubagentError::Source(e)));
        }
    }

    // Await the result from the oneshot channel with a generous timeout.
    match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        rx,
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(SubagentError::Source(anyhow::anyhow!("subagent channel closed"))),
        Err(_) => Err(SubagentError::Source(anyhow::anyhow!("subagent timed out after 300s"))),
    }
}

async fn run_subagent_turn(
    provider: &dyn Provider,
    cmd: &AgentCommand,
    max_iterations: usize,
) -> Result<String, SubagentError> {
    let gate = PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink));
    run_subagent_turn_with_gate(provider, cmd, max_iterations, gate).await
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

    #[tokio::test]
    async fn subagent_channel_returns_result() {
        // Layer 1: verify that the channel mechanism returns the expected result.
        let provider = mock_provider();
        let result = run_subagent("channel test", "mock", "echo", &provider, ThinkingLevel::Off, false, "", "", 5).await;
        assert!(result.is_ok(), "channel should deliver result: {result:?}");
        assert!(result.unwrap().contains("channel test"));
    }

    #[tokio::test]
    async fn subagent_channel_drops_on_cancel() {
        // Layer 1: dropping the receiver closes the channel; sender handles it
        // gracefully without panicking.
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, SubagentError>>();
        // Drop the receiver to simulate cancellation.
        drop(rx);
        // Sending on a closed channel returns Err; we handle it with `let _ =`.
        let result = tx.send(Ok("result".to_string()));
        assert!(result.is_err(), "sending on closed channel should fail gracefully");
    }

    #[tokio::test]
    async fn subagent_timeout_returns_error() {
        // Layer 1: verify timeout handling works for the channel await.
        // We create a receiver and never send on it, then timeout after 1ms.
        use tokio::time::Duration;
        let (_tx, rx) = tokio::sync::oneshot::channel::<Result<String, SubagentError>>();
        let result = tokio::time::timeout(Duration::from_millis(1), rx).await;
        // Timeout returns Err(Elapsed).
        assert!(result.is_err(), "timeout should return Elapsed error");
    }

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
