use crate::parser::{parse_tool_calls, ParsedToolCall};
use crate::AgentCommand;
use anyhow::Result;
use futures::StreamExt;
use runie_core::event::AgentEvent;
use runie_core::event::Event;
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
) -> Result<()> {
    run_agent_turn_with_skills(provider, command, emit, max_iterations, None).await
}

/// Run an agent turn with explicit skill registry.
pub async fn run_agent_turn_with_skills(
    provider: &DynProvider,
    command: &AgentCommand,
    emit: EmitFn,
    max_iterations: usize,
    skills: Option<&SkillRegistry>,
) -> Result<()> {
    let mut messages = build_initial_messages(command);
    let turn_start = Instant::now();
    let mut tool_call_count = 0;

    // Check turn start hooks
    if let Some(skills) = skills {
        if let TurnStartResult::SkipWithMessage(msg) = skills.on_turn_start(&TurnStartCtx {
            message: command.content.clone(),
            system_prompt: command.system_prompt.clone(),
            skills_context: command.skills_context.clone(),
        }) {
            emit_response_and_done(&emit, &command.id, msg);
            return Ok(());
        }
        if let TurnStartResult::Abort(reason) = skills.on_turn_start(&TurnStartCtx {
            message: command.content.clone(),
            system_prompt: command.system_prompt.clone(),
            skills_context: command.skills_context.clone(),
        }) {
            emit_error_and_done(&emit, &command.id, format!("Turn aborted by skill: {}", reason));
            return Ok(());
        }
    }

    let has_intermediate_steps = run_iterations(
        provider, command, &mut messages, emit.clone(), skills, max_iterations, &mut tool_call_count,
    )
    .await?;

    emit_turn_end(&emit, &command.id, skills, &messages, tool_call_count, has_intermediate_steps, turn_start);
    Ok(())
}

/// Emit final turn end events including on_turn_end hook.
fn emit_turn_end(
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

        let ctx = TurnEndCtx { assistant_message: assistant_msg, tool_call_count, success: true };
        match skills.on_turn_end(&ctx) {
            TurnEndResult::Continue | TurnEndResult::Abort(_) => {}
            TurnEndResult::RequestAnotherPass => {}
        }
    }

    if has_intermediate_steps {
        emit_now(emit, AgentEvent::TurnComplete {
            id: id.to_string(),
            duration_secs: turn_start.elapsed().as_secs_f64(),
        });
    }
    emit_now(emit, AgentEvent::Done { id: id.to_string() });
}

fn emit_response_and_done(emit: &EmitFn, id: &str, content: String) {
    emit_now(emit, AgentEvent::Response { id: id.to_string(), content });
    emit_now(emit, AgentEvent::Done { id: id.to_string() });
}

fn emit_error_and_done(emit: &EmitFn, id: &str, message: String) {
    emit_now(emit, AgentEvent::Error { id: id.to_string(), message });
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
) -> Result<bool> {
    let mut has_intermediate_steps = false;
    for _ in 0..max_iterations {
        if !run_agent_iteration(provider, command, messages, emit.clone(), skills, tool_call_count).await? {
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
) -> Result<bool> {
    emit_now(&emit, AgentEvent::Thinking { id: command.id.clone() });

    let response_text = stream_response(provider, command, messages, emit.clone()).await?;
    emit_now(&emit, AgentEvent::ThoughtDone { id: command.id.clone() });

    let tools = parse_tool_calls(&response_text);
    if tools.is_empty() {
        return Ok(false);
    }

    messages.push(ChatMessage::assistant(response_text));
    execute_tools(&command.id, &tools, emit, messages, skills, tool_call_count).await;
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
                emit_now(&emit, AgentEvent::ResponseDelta {
                    id: command.id.clone(),
                    content: text,
                });
            }
            runie_core::llm_event::LLMEvent::Finish { .. } => break,
            runie_core::llm_event::LLMEvent::Error(e) => {
                return Err(anyhow::anyhow!("LLM error: {:?}", e));
            }
            _ => {}
        }
    }
    emit_now(&emit, AgentEvent::Response {
        id: command.id.clone(),
        content: response_text.clone(),
    });
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
) {
    let ctx = ToolContext::default();
    let registry = runie_core::tool::builtin_registry();

    for tool_call in tools {
        *tool_call_count += 1;
        let output = execute_single_tool(cmd_id, tool_call, emit.clone(), skills, &ctx, &registry).await;

        emit_now(&emit, AgentEvent::ToolEnd {
            id: cmd_id.to_string(),
            duration_secs: output.duration.as_secs_f64(),
            output: output.content.clone(),
        });
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
) -> ToolOutput {
    emit_now(&emit, AgentEvent::ToolStart {
        id: cmd_id.to_string(),
        name: tool_call.name.clone(),
        input: tool_call.args.clone(),
    });

    // Check skill Before hook
    let skill_override = check_tool_call_before_hook(skills, tool_call);
    if let Some(output) = skill_override {
        return ToolOutput {
            tool_name: tool_call.name.clone(),
            tool_args: tool_call.args.clone(),
            content: output,
            bytes_transferred: None,
            duration: std::time::Duration::from_millis(0),
            status: ToolStatus::Success,
        };
    }

    let output = execute_tool_call(registry, tool_call, ctx).await;

    // Fire skill After hook
    if let Some(skills) = skills {
        skills.on_tool_call(&ToolCallCtx {
            tool_name: tool_call.name.clone(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::After,
            tool_output: Some(output.content.clone()),
            success: Some(output.status == ToolStatus::Success),
        });
    }

    output
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
) -> ToolOutput {
    let tool_name = &tool_call.name;

    match registry.get(tool_name) {
        Some(tool) => tool.call(tool_call.args.clone(), ctx).await.unwrap_or_else(|e| {
            ToolOutput {
                tool_name: tool_name.clone(),
                tool_args: tool_call.args.clone(),
                content: format!("Tool execution failed: {}", e),
                bytes_transferred: None,
                duration: std::time::Duration::from_millis(0),
                status: ToolStatus::Error,
            }
        }),
        None => ToolOutput {
            tool_name: tool_name.clone(),
            tool_args: tool_call.args.clone(),
            content: format!("Error: unknown tool '{}'", tool_name),
            bytes_transferred: None,
            duration: std::time::Duration::from_millis(0),
            status: ToolStatus::Error,
        },
    }
}
