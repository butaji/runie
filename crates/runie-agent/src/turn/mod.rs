//! Agent turn execution.

use crate::stream_response::{stream_response, EmitFn, StreamedResponse};
use crate::AgentCommand;
use anyhow::Result;
use runie_core::harness_skills::{
    SkillRegistry, TurnEndCtx, TurnEndResult, TurnStartCtx, TurnStartResult,
};
use runie_core::message::{ChatMessage, Role};
use runie_core::permissions::PermissionGate;
use runie_core::provider::Provider;
use runie_core::sanitize::sanitize_messages;
use runie_core::tool::ToolRegistry;
use runie_core::tool::{
    assign_tool_call_ids, build_assistant_message, tool_parse_error_message, ParsedToolCall,
};
use std::time::Instant;

// Helper modules
mod emit;
mod tools;

use emit::{emit_error_and_done, emit_now, emit_response_and_done};
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
    let has_intermediate_steps = run_iterations(
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
        has_intermediate_steps,
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
    has_intermediate_steps: bool,
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

    if has_intermediate_steps {
        emit_now(
            emit,
            runie_core::Event::TurnComplete {
                id: id.to_owned(),
                duration_secs: turn_start.elapsed().as_secs_f64(),
            },
        );
    }
    emit_now(emit, runie_core::Event::Done { id: id.to_owned() });
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
) -> Result<bool> {
    let mut has_intermediate_steps = false;
    for _ in 0..max_iterations {
        if !run_agent_iteration(
            provider,
            command,
            messages,
            emit.clone(),
            skills,
            tool_call_count,
            &gate,
        )
        .await?
        {
            break;
        }
        has_intermediate_steps = true;
    }
    Ok(has_intermediate_steps)
}

async fn run_agent_iteration(
    provider: &dyn Provider,
    command: &AgentCommand,
    messages: &mut Vec<ChatMessage>,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    tool_call_count: &mut usize,
    gate: &PermissionGate,
) -> Result<bool> {
    emit_now(
        &emit,
        runie_core::Event::Thinking {
            id: command.id.clone(),
        },
    );
    let tools = build_tool_registry(command.read_only).to_openai_functions();
    let response = stream_response(provider, &command.id, messages, tools, emit.clone()).await?;
    emit_now(
        &emit,
        runie_core::Event::ThoughtDone {
            id: command.id.clone(),
        },
    );
    if response.tool_calls.is_empty() {
        return Ok(false);
    }
    let tools = collect_parsed_tool_calls(&response, messages);
    execute_tools(
        &command.id,
        &tools,
        emit,
        messages,
        skills,
        tool_call_count,
        gate,
    )
    .await;
    sanitize_messages(messages);
    Ok(true)
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

fn build_tool_registry(read_only: bool) -> ToolRegistry {
    let registry = crate::tool::builtin_registry();
    if read_only {
        registry.read_only_subset()
    } else {
        registry
    }
}

pub(crate) fn build_initial_messages(command: &AgentCommand) -> Vec<ChatMessage> {
    let tools_list = if command.read_only {
        "read_file, list_dir, grep, find, search, fetch_docs"
    } else {
        "read_file, list_dir, write_file, edit_file, bash, grep, find, search, fetch_docs"
    };
    let base = if command.system_prompt.is_empty() {
        runie_core::prompts::DEFAULT_PROMPT
    } else {
        &command.system_prompt
    };
    let mut system = runie_core::prompts::build_system_prompt(
        base,
        tools_list,
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

