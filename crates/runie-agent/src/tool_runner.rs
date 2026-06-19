//! Shared tool execution helpers for agent turn and headless runners.

use runie_core::tool_parser::ParsedToolCall;
use crate::PermissionGate;
use runie_core::message::{ChatMessage, Part};
use runie_core::permissions::{PermissionAction, PermissionContext};
use runie_core::tool::{ToolContext, ToolOutput, ToolStatus};

/// Execute a single parsed tool call, respecting the permission gate.
pub async fn execute_tool_call(
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
                PermissionAction::Deny | PermissionAction::Ask => {
                    blocked_output(tool_name, tool_call)
                }
            }
        }
        None => unknown_tool_output(tool_name, tool_call),
    }
}

pub fn build_permission_context<'a>(
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

async fn run_tool(
    tool: &std::sync::Arc<dyn runie_core::tool::Tool>,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
) -> ToolOutput {
    tool.call(tool_call.args.clone(), ctx)
        .await
        .unwrap_or_else(|e| ToolOutput {
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

/// Build a tool-result chat message carrying the matching tool-call id.
pub fn tool_result_message(tool_call: &ParsedToolCall, output: &ToolOutput) -> ChatMessage {
    let id = tool_call.id.clone().unwrap_or_default();
    ChatMessage::tool_result(format!("{} result:\n{}", tool_call.name, output.content))
        .with_tool_call_id(id.clone())
        .with_parts(vec![Part::tool_result(id, &output.content)])
}
