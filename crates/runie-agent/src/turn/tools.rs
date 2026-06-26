//! Tool execution helpers for agent turn.

use crate::tool_runner::execute_tool_call;
use crate::turn::emit::emit_now;
use runie_core::harness_skills::{
    SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult,
};
use runie_core::message::ChatMessage;
use runie_core::permissions::PermissionGate;
use runie_core::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::tool::ParsedToolCall;
use crate::stream_response::EmitFn;

pub async fn execute_tools(
    cmd_id: &str,
    tools: &[ParsedToolCall],
    emit: EmitFn,
    messages: &mut Vec<ChatMessage>,
    skills: Option<&SkillRegistry>,
    tool_call_count: &mut usize,
    gate: &PermissionGate,
) {
    let ctx = ToolContext::default();
    let registry = crate::tool::builtin_registry();

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
            runie_core::Event::ToolEnd {
                id: cmd_id.to_owned(),
                duration_secs: output.duration.as_secs_f64(),
                output: output.content.clone(),
            },
        );
        messages.push(crate::tool_runner::tool_result_message(tool_call, &output));
    }
}

pub async fn execute_single_tool(
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
        runie_core::Event::ToolStart {
            id: cmd_id.to_owned(),
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
