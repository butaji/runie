//! Tool execution helpers for agent turn.

use crate::stream_response::EmitFn;
use crate::tool_runner::{
    execute_tool_call, fire_skill_after_hook, run_skill_before_hook, tool_result_message,
};

use runie_core::harness_skills::SkillRegistry;
use runie_core::message::ChatMessage;
use runie_core::permissions::PermissionGate;
use runie_core::tool::ParsedToolCall;
use runie_core::tool::{ToolContext, ToolOutput};

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

    for tool_call in tools {
        *tool_call_count += 1;
        let tool_id = tool_call.id.as_deref().unwrap_or(cmd_id);
        let output =
            execute_single_tool(tool_id, tool_call, emit.clone(), skills, &ctx, gate).await;

        emit(runie_core::Event::ToolEnd {
            id: tool_id.to_owned(),
            duration_secs: output.duration.as_secs_f64(),
            output: output.content.clone(),
        });
        messages.push(tool_result_message(tool_call, &output));
    }
}

pub async fn execute_single_tool(
    tool_id: &str,
    tool_call: &ParsedToolCall,
    emit: EmitFn,
    skills: Option<&SkillRegistry>,
    ctx: &ToolContext,
    gate: &PermissionGate,
) -> ToolOutput {
    emit_tool_start(tool_id, tool_call, &emit);

    if let Some(output) = run_skill_before_hook(skills, tool_call) {
        return output;
    }

    let output = execute_tool_call(tool_call, ctx, gate, None).await;
    fire_skill_after_hook(skills, tool_call, &output);
    output
}

fn emit_tool_start(tool_id: &str, tool_call: &ParsedToolCall, emit: &EmitFn) {
    emit(runie_core::Event::ToolStart {
        id: tool_id.to_owned(),
        name: tool_call.name.clone(),
        input: tool_call.args.clone(),
    });
}
