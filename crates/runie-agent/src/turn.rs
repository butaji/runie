use crate::parser::{parse_tool_calls, ParsedToolCall};
use crate::permission_gate::PermissionGate;
use crate::AgentCommand;
use anyhow::Result;
use futures::StreamExt;
use runie_core::event::{AgentEvent, Event};
use runie_core::permissions::{PermissionAction, PermissionContext};
use runie_core::harness_skills::{
    SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult, TurnEndCtx, TurnEndResult,
    TurnStartCtx, TurnStartResult,
};
use runie_core::message::{ChatMessage, Role};
use runie_core::provider::Provider;
use runie_core::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_provider::DynProvider;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Emit type: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>
type EmitFn = Arc<Mutex<dyn FnMut(Event) + Send + Sync>>;

/// Run an agent turn with optional skill hooks.
pub async fn run_agent_turn(
    provider: &DynProvider,
    command: &AgentCommand,
    emit: EmitFn,
    max_iterations: usize,
    gate: PermissionGate,
) -> Result<()> {
    run_agent_turn_with_skills(provider, command, emit, max_iterations, None, gate).await
}

/// Run an agent turn with explicit skill registry.
pub async fn run_agent_turn_with_skills(
    provider: &DynProvider,
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

    finalize_turn(&emit, command, skills, &messages, tool_call_count, has_intermediate_steps, turn_start).await;
    Ok(())
}

async fn finalize_turn(
    emit: &EmitFn,
    command: &AgentCommand,
    skills: Option<&SkillRegistry>,
    messages: &[ChatMessage],
    tool_call_count: usize,
    has_intermediate_steps: bool,
    turn_start: Instant,
) {
    emit_turn_end(
        emit,
        &command.id,
        skills,
        messages,
        tool_call_count,
        has_intermediate_steps,
        turn_start,
    ).await;
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

    if let TurnStartResult::SkipWithMessage(msg) = skills.on_turn_start(&ctx) {
        emit_response_and_done(emit, &command.id, msg);
        return Some(Ok(()));
    }
    if let TurnStartResult::Abort(reason) = skills.on_turn_start(&ctx) {
        emit_error_and_done(
            emit,
            &command.id,
            format!("Turn aborted by skill: {}", reason),
        );
        return Some(Ok(()));
    }
    None
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
            .map(|m| m.content.clone())
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
            AgentEvent::TurnComplete {
                id: id.to_string(),
                duration_secs: turn_start.elapsed().as_secs_f64(),
            },
        );
    }
    emit_now(emit, AgentEvent::Done { id: id.to_string() });
}

fn emit_response_and_done(emit: &EmitFn, id: &str, content: String) {
    emit_now(
        emit,
        AgentEvent::Response {
            id: id.to_string(),
            content,
        },
    );
    emit_now(emit, AgentEvent::Done { id: id.to_string() });
}

fn emit_error_and_done(emit: &EmitFn, id: &str, message: String) {
    emit_now(
        emit,
        AgentEvent::Error {
            id: id.to_string(),
            message,
        },
    );
    emit_now(emit, AgentEvent::Done { id: id.to_string() });
}

async fn run_iterations(
    provider: &DynProvider,
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

fn emit_now(emit: &EmitFn, event: Event) {
    emit.lock().unwrap()(event);
}

async fn run_agent_iteration(
    provider: &DynProvider,
    command: &AgentCommand,
    messages: &mut Vec<ChatMessage>,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    tool_call_count: &mut usize,
    gate: &PermissionGate,
) -> Result<bool> {
    emit_now(
        &emit,
        AgentEvent::Thinking {
            id: command.id.clone(),
        },
    );

    let response_text = stream_response(provider, command, messages, emit.clone()).await?;
    emit_now(
        &emit,
        AgentEvent::ThoughtDone {
            id: command.id.clone(),
        },
    );

    let tools = parse_tool_calls(&response_text);
    if tools.is_empty() {
        return Ok(false);
    }

    messages.push(ChatMessage::assistant(response_text));
    execute_tools(&command.id, &tools, emit, messages, skills, tool_call_count, gate).await;
    Ok(true)
}

async fn stream_response(
    provider: &DynProvider,
    command: &AgentCommand,
    messages: &[ChatMessage],
    emit: EmitFn,
) -> Result<String> {
    let mut response_text = String::new();
    let mut stream = provider.generate(messages.to_vec());
    while let Some(event_result) = stream.next().await {
        let event = event_result?;
        match event {
            runie_core::llm_event::LLMEvent::TextDelta(text) => {
                response_text.push_str(&text);
                emit_now(
                    &emit,
                    AgentEvent::ResponseDelta {
                        id: command.id.clone(),
                        content: text,
                    },
                );
            }
            runie_core::llm_event::LLMEvent::Finish { .. } => break,
            runie_core::llm_event::LLMEvent::Error(e) => {
                return Err(anyhow::anyhow!("LLM error: {:?}", e));
            }
            _ => {}
        }
    }
    emit_now(
        &emit,
        AgentEvent::Response {
            id: command.id.clone(),
            content: response_text.clone(),
        },
    );
    Ok(response_text)
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

async fn execute_tools(
    cmd_id: &str,
    tools: &[ParsedToolCall],
    emit: EmitFn,
    messages: &mut Vec<ChatMessage>,
    skills: Option<&SkillRegistry>,
    tool_call_count: &mut usize,
    gate: &PermissionGate,
) {
    let ctx = ToolContext::default();
    let registry = runie_engine::tool::builtin_registry();

    for tool_call in tools {
        *tool_call_count += 1;
        let output = execute_single_tool(
            cmd_id, tool_call, emit.clone(), skills, &ctx, &registry, gate,
        )
        .await;

        emit_now(
            &emit,
            AgentEvent::ToolEnd {
                id: cmd_id.to_string(),
                duration_secs: output.duration.as_secs_f64(),
                output: output.content.clone(),
            },
        );
        messages.push(ChatMessage::tool_result(format!(
            "{} result:\n{}",
            tool_call.name, output.content
        )));
    }
}

async fn execute_single_tool(
    cmd_id: &str,
    tool_call: &ParsedToolCall,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    ctx: &ToolContext,
    registry: &runie_core::tool::ToolRegistry,
    gate: &PermissionGate,
) -> ToolOutput {
    emit_tool_start(cmd_id, tool_call, &emit);

    if let Some(output) = skill_override_output(skills, tool_call) {
        return output;
    }

    let output = execute_tool_call(registry, tool_call, ctx, gate).await;
    fire_tool_after_hook(skills, tool_call, &output);

    output
}

fn emit_tool_start(cmd_id: &str, tool_call: &ParsedToolCall, emit: &EmitFn) {
    emit_now(
        emit,
        AgentEvent::ToolStart {
            id: cmd_id.to_string(),
            name: tool_call.name.clone(),
            input: tool_call.args.clone(),
        },
    );
}

fn skill_override_output(
    skills: Option<&SkillRegistry>,
    tool_call: &ParsedToolCall,
) -> Option<ToolOutput> {
    check_tool_call_before_hook(skills, tool_call).map(|output| ToolOutput {
        tool_name: tool_call.name.clone(),
        tool_args: tool_call.args.clone(),
        content: output,
        bytes_transferred: None,
        duration: std::time::Duration::from_millis(0),
        status: ToolStatus::Success,
    })
}

fn fire_tool_after_hook(
    skills: Option<&SkillRegistry>,
    tool_call: &ParsedToolCall,
    output: &ToolOutput,
) {
    if let Some(skills) = skills {
        skills.on_tool_call(&ToolCallCtx {
            tool_name: tool_call.name.clone(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::After,
            tool_output: Some(output.content.clone()),
            success: Some(output.status == ToolStatus::Success),
        });
    }
}

fn check_tool_call_before_hook(
    skills: Option<&SkillRegistry>,
    tool_call: &ParsedToolCall,
) -> Option<String> {
    let skills = skills?;

    let tool_ctx = ToolCallCtx {
        tool_name: tool_call.name.clone(),
        tool_input: serde_json::json!({}),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    };

    match skills.on_tool_call(&tool_ctx) {
        ToolCallResult::Continue => None,
        ToolCallResult::SkipWithOutput(output) => Some(output),
        ToolCallResult::Abort(_reason) => {
            panic!("Tool abort not implemented in this path");
        }
    }
}

async fn execute_tool_call(
    registry: &runie_core::tool::ToolRegistry,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
) -> ToolOutput {
    let tool_name = &tool_call.name;

    match registry.get(tool_name) {
        Some(tool) => {
            let perm_ctx = build_permission_context(tool_name, &tool_call.args, &ctx.working_dir);
            match gate.evaluate(&perm_ctx).await {
                PermissionAction::Allow => run_tool(tool, tool_call, ctx).await,
                PermissionAction::Deny | PermissionAction::Ask => blocked_output(tool_name, tool_call),
            }
        }
        None => unknown_tool_output(tool_name, tool_call),
    }
}

async fn run_tool(
    tool: &std::sync::Arc<dyn runie_core::tool::Tool>,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
) -> ToolOutput {
    tool.call(tool_call.args.clone(), ctx).await.unwrap_or_else(|e| ToolOutput {
        tool_name: tool_call.name.clone(),
        tool_args: tool_call.args.clone(),
        content: format!("Tool execution failed: {}", e),
        bytes_transferred: None,
        duration: std::time::Duration::from_millis(0),
        status: ToolStatus::Error,
    })
}

fn blocked_output(tool_name: &str, tool_call: &ParsedToolCall) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_string(),
        tool_args: tool_call.args.clone(),
        content: format!("Permission denied for tool '{}'", tool_name),
        bytes_transferred: None,
        duration: std::time::Duration::from_millis(0),
        status: ToolStatus::Blocked,
    }
}

fn unknown_tool_output(tool_name: &str, tool_call: &ParsedToolCall) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_string(),
        tool_args: tool_call.args.clone(),
        content: format!("Error: unknown tool '{}'", tool_name),
        bytes_transferred: None,
        duration: std::time::Duration::from_millis(0),
        status: ToolStatus::Error,
    }
}

fn build_permission_context<'a>(
    tool: &'a str,
    input: &'a serde_json::Value,
    cwd: &'a std::path::Path,
) -> PermissionContext<'a> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::Path::new);
    PermissionContext {
        tool,
        path,
        input: Some(input),
        cwd: Some(cwd),
    }
}
