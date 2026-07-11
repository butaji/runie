//! Agent turn execution.

use crate::stream_response::{stream_response, EmitFn, StreamedResponse};
use crate::tool_registry::build_schemas as build_tool_registry;
use crate::AgentCommand;
use anyhow::Result;
use runie_core::event::Event;
use runie_core::harness_skills::{
    SkillRegistry, TurnEndCtx, TurnEndResult, TurnStartCtx, TurnStartResult,
};
use runie_core::message::{ChatMessage, Role};
use runie_core::permissions::PermissionGate;
use runie_core::provider::Provider;
use runie_core::sanitize::sanitize_messages;
use runie_core::tool::BUILTIN_TOOL_NAMES;
use runie_core::tool::{
    assign_tool_call_ids, build_assistant_message, tool_parse_error_message, ParsedToolCall,
};
use std::time::Instant;

// Helper modules
mod emit;
mod tools;

use emit::{emit_error_and_done, emit_response_and_done};
use tools::execute_tools;

/// Run an agent turn with optional skill hooks.
pub async fn run_agent_turn(
    provider: &dyn Provider,
    command: &AgentCommand,
    emit: EmitFn,
    max_iterations: usize,
    gate: PermissionGate,
) -> Result<()> {
    run_agent_turn_with_skills(provider, command, emit, max_iterations, None, gate).await
}

/// Run an agent turn with explicit skill registry.
pub async fn run_agent_turn_with_skills(
    provider: &dyn Provider,
    command: &AgentCommand,
    emit: EmitFn,
    max_iterations: usize,
    skills: Option<&SkillRegistry>,
    gate: PermissionGate,
) -> Result<()> {
    let mut messages = build_initial_messages(command);
    let turn_start = Instant::now();
    let mut tool_call_count = 0;
    if let Some(skills) = skills {
        if let Some(result) = check_turn_start(skills, command, &emit) {
            return result;
        }
    }
    run_iterations(
        provider,
        command,
        &mut messages,
        emit.clone(),
        skills,
        max_iterations,
        &mut tool_call_count,
        gate,
    )
    .await?;

    emit_turn_end(
        &emit,
        &command.id,
        skills,
        &messages,
        tool_call_count,
        turn_start,
    )
    .await;
    Ok(())
}

fn check_turn_start(
    skills: &SkillRegistry,
    command: &AgentCommand,
    emit: &EmitFn,
) -> Option<Result<()>> {
    let ctx = TurnStartCtx {
        message: command.content.clone(),
        system_prompt: command.system_prompt.clone(),
        skills_context: command.skills_context.clone(),
    };

    match skills.on_turn_start(&ctx) {
        TurnStartResult::SkipWithMessage(msg) => {
            emit_response_and_done(emit, &command.id, msg);
            Some(Ok(()))
        }
        TurnStartResult::Abort(reason) => {
            emit_error_and_done(
                emit,
                &command.id,
                format!("Turn aborted by skill: {}", reason),
            );
            Some(Ok(()))
        }
        TurnStartResult::Continue => None,
    }
}

/// Emit final turn end events including on_turn_end hook.
async fn emit_turn_end(
    emit: &EmitFn,
    id: &str,
    skills: Option<&SkillRegistry>,
    messages: &[ChatMessage],
    tool_call_count: usize,
    turn_start: Instant,
) {
    if let Some(skills) = skills {
        let assistant_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .map(|m| m.content())
            .unwrap_or_default();

        let ctx = TurnEndCtx {
            assistant_message: assistant_msg,
            tool_call_count,
            success: true,
        };
        match skills.on_turn_end(&ctx).await {
            TurnEndResult::Continue | TurnEndResult::Abort(_) => {}
            TurnEndResult::RequestAnotherPass => {}
        }
    }

    emit(Event::TurnComplete {
        id: id.to_owned(),
        duration_secs: turn_start.elapsed().as_secs_f64(),
    });
    emit(Event::Done { id: id.to_owned() });
}

// allow: iteration control params — orthogonal and intentionally flat for turn loop clarity
#[allow(clippy::too_many_arguments)]
async fn run_iterations(
    provider: &dyn Provider,
    command: &AgentCommand,
    messages: &mut Vec<ChatMessage>,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    max_iterations: usize,
    tool_call_count: &mut usize,
    gate: PermissionGate,
) -> Result<()> {
    // Signature of the tool-call set executed in the immediately previous
    // iteration. Used to detect a model re-issuing the exact same call and stop
    // the loop instead of spinning to `max_iterations`.
    let mut last_tool_signature: Option<Vec<(String, String)>> = None;
    for _ in 0..max_iterations {
        if !run_agent_iteration(
            provider,
            command,
            messages,
            emit.clone(),
            skills,
            tool_call_count,
            &gate,
            &mut last_tool_signature,
        )
        .await?
        {
            break;
        }
    }
    Ok(())
}

// allow: `last_tool_signature` threads the repeat-guard state across iterations.
#[allow(clippy::too_many_arguments)]
async fn run_agent_iteration(
    provider: &dyn Provider,
    command: &AgentCommand,
    messages: &mut Vec<ChatMessage>,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    tool_call_count: &mut usize,
    gate: &PermissionGate,
    last_tool_signature: &mut Option<Vec<(String, String)>>,
) -> Result<bool> {
    emit(Event::Thinking {
        id: command.id.clone(),
    });
    let tools = build_tool_registry(command.read_only);
    let cancel_token = command.cancellation_token.clone();
    let response = match stream_response(
        provider,
        &command.id,
        messages,
        tools,
        emit.clone(),
        cancel_token,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            emit(Event::ThoughtDone {
                id: command.id.clone(),
            });
            return Err(e);
        }
    };
    emit(Event::ThoughtDone {
        id: command.id.clone(),
    });
    if response.tool_calls.is_empty() {
        return Ok(false);
    }
    let tools = collect_parsed_tool_calls(&response, messages);

    // Repeat guard: if the model re-issued the exact same set of (name, args)
    // tool calls as the immediately previous iteration in this turn, stop
    // instead of re-executing. This prevents an infinite re-issue loop even
    // when the tool is allowed.
    let signature = tool_call_signature(&tools);
    if last_tool_signature.as_ref() == Some(&signature) {
        return Ok(false);
    }

    let any_blocked = execute_tools(
        &command.id,
        &tools,
        emit,
        messages,
        skills,
        tool_call_count,
        gate,
    )
    .await;
    *last_tool_signature = Some(signature);
    sanitize_messages(messages);

    // Stop the loop if any tool was blocked (denied by the permission gate).
    // Mirrors the headless runner: the agent must not re-issue a tool call
    // after a denial.
    if any_blocked {
        return Ok(false);
    }
    Ok(true)
}

/// Canonical, order-independent signature of a tool-call set: sorted
/// `(name, serialized_args)` pairs. Used to detect a repeated call across
/// consecutive iterations.
fn tool_call_signature(tools: &[ParsedToolCall]) -> Vec<(String, String)> {
    let mut sig: Vec<(String, String)> = tools
        .iter()
        .map(|t| {
            let args = serde_json::to_string(&t.args).unwrap_or_default();
            (t.name.clone(), args)
        })
        .collect();
    sig.sort();
    sig
}

fn collect_parsed_tool_calls(
    response: &StreamedResponse,
    messages: &mut Vec<ChatMessage>,
) -> Vec<ParsedToolCall> {
    let mut tools = response.tool_calls.clone();
    assign_tool_call_ids(&mut tools);
    messages.push(build_assistant_message(
        &response.text,
        response.reasoning.as_deref(),
        &tools,
    ));
    for (i, err) in response.parse_errors.iter().enumerate() {
        messages.push(tool_parse_error_message(err, &format!("parse_{}", i)));
    }
    tools
}

/// Tools that require write permissions (filtered in read-only mode).
const WRITE_TOOLS: &[&str] = crate::tool_registry::WRITE_TOOL_NAMES;

/// Build the comma-separated tools-list string from BUILTIN_TOOL_NAMES.
/// Read-only tools are filtered out when `read_only` is true.
fn build_tools_list(read_only: bool) -> String {
    BUILTIN_TOOL_NAMES
        .iter()
        .filter(|name| !read_only || !WRITE_TOOLS.contains(name))
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn build_initial_messages(command: &AgentCommand) -> Vec<ChatMessage> {
    let tools_list = build_tools_list(command.read_only);
    let base = if command.system_prompt.is_empty() {
        runie_core::prompts::DEFAULT_PROMPT
    } else {
        &command.system_prompt
    };
    let mut system = runie_core::prompts::build_system_prompt(
        base,
        &tools_list,
        command.read_only,
        command.thinking_level.prompt_suffix(),
    );
    if !command.skills_context.is_empty() {
        system.push_str(&command.skills_context);
    }
    vec![
        ChatMessage::system(system),
        ChatMessage::user(command.content.clone()),
    ]
}

// Re-export emit and tools for internal use
