use runie_core::sanitize::sanitize_messages;
use runie_core::tool_parser::{
    assign_tool_call_ids, build_assistant_message, tool_parse_error_message, ParsedToolCall,
};
use runie_core::permissions::PermissionGate;
use crate::stream_response::{stream_response, EmitFn, StreamedResponse};
use crate::tool_runner::{execute_tool_call, tool_result_message};
use crate::AgentCommand;
use anyhow::Result;
use runie_core::event::{AgentEvent, Event};
use runie_core::harness_skills::{
    SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult, TurnEndCtx, TurnEndResult,
    TurnStartCtx, TurnStartResult,
};
use runie_core::message::{ChatMessage, Role};
use runie_core::provider::Provider;
use runie_core::tool::{ToolContext, ToolOutput, ToolRegistry, ToolStatus};
use std::time::Instant;

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
            .map(|m| m.content().clone())
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

fn emit_now(emit: &EmitFn, event: Event) {
    let mut emit = emit.lock().unwrap_or_else(|p| p.into_inner());
    emit(event);
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
    emit_now(&emit, AgentEvent::Thinking { id: command.id.clone() });
    let tools = build_tool_registry(command.read_only).to_openai_functions();
    let response = stream_response(provider, &command.id, messages, tools, emit.clone()).await?;
    emit_now(&emit, AgentEvent::ThoughtDone { id: command.id.clone() });
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
    let registry = runie_engine::tool::builtin_registry();
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
            cmd_id,
            tool_call,
            emit.clone(),
            skills,
            &ctx,
            &registry,
            gate,
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
        messages.push(tool_result_message(tool_call, &output));
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
            tool_input: tool_call.args.clone(),
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
        tool_input: tool_call.args.clone(),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    };

    match skills.on_tool_call(&tool_ctx) {
        ToolCallResult::Continue => None,
        ToolCallResult::SkipWithOutput(output) => Some(output),
        ToolCallResult::Abort(reason) => {
            Some(format!("Tool {} aborted: {}", tool_call.name, reason))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::harness_skills::HarnessSkill;
    use std::sync::{Arc, Mutex};

    #[test]
    fn emit_now_recovers_from_poisoned_mutex() {
        let emit: EmitFn = Arc::new(Mutex::new(|_| {}));
        let emit2 = emit.clone();
        let handle = std::thread::spawn(move || {
            let _guard = emit2.lock().unwrap();
            panic!("poison emit mutex")
        });
        let _ = handle.join();

        // Should not panic despite the poisoned mutex.
        emit_now(&emit, Event::Abort);
    }

    /// Mock skill that counts on_turn_start calls.
    struct CountingSkill {
        count: Arc<Mutex<usize>>,
    }

    impl CountingSkill {
        fn new(count: Arc<Mutex<usize>>) -> Self {
            Self { count }
        }
    }

    impl HarnessSkill for CountingSkill {
        fn name(&self) -> &str {
            "counting"
        }

        fn on_turn_start(&self, _ctx: &TurnStartCtx) -> TurnStartResult {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            TurnStartResult::Continue
        }
    }

    #[test]
    fn turn_start_hook_called_once() {
        let count = Arc::new(Mutex::new(0));
        let mut registry = SkillRegistry::new();
        registry.register(CountingSkill::new(count.clone()));

        let cmd = AgentCommand {
            content: "test".to_string(),
            id: "req.0".to_string(),
            provider: "mock".to_string(),
            model: "echo".to_string(),
            thinking_level: runie_core::model::ThinkingLevel::Off,
            read_only: false,
            skills_context: String::new(),
            system_prompt: String::new(),
            truncation: crate::truncate::TruncationPolicy::default(),
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| {}));

        let result = check_turn_start(&registry, &cmd, &emit);
        assert!(result.is_none(), "check_turn_start should return None for Continue");

        let call_count = *count.lock().unwrap();
        assert_eq!(call_count, 1, "on_turn_start should be called exactly once");
    }

    // ─── Layer 2 ──────────────────────────────────────────────────────────────
    /// Verify sanitize_messages is called after tool execution: a dangling tool
    /// call without a matching result must be removed.
    #[tokio::test]
    async fn agent_turn_calls_sanitize_after_tool_execution() {
        let mut messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("list files"),
            ChatMessage::assistant("").with_tool_calls(vec![runie_core::message::ToolCall {
                id: "dangling_call".to_string(),
                name: "list_dir".to_string(),
                args: serde_json::json!({"path": "."}),
            }]),
        ];
        // Simulate sanitize run after execute_tools (no result pushed).
        sanitize_messages(&mut messages);
        let assistant = messages.iter().find(|m| m.role == runie_core::message::Role::Assistant);
        assert!(
            assistant.map(|m| m.tool_calls().is_empty()).unwrap_or(false),
            "Dangling tool call should be removed by sanitize_messages"
        );
    }
}
