//! Tool execution helpers for rig-based agent loop.

use crate::events::{AgentEvent, AgentMessage, ContentPart, ToolResult};
use crate::{Hook, HookDecision};
use crate::rig_loop::AgentLoopError;
use runie_core::{Context, Message, ToolCall as CoreToolCall};
use runie_tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Converts AgentMessage to runie_core::Message.
pub(crate) fn agent_message_to_core(msg: &AgentMessage) -> Option<Message> {
    let content = msg.content.iter().filter_map(|part| match part {
        ContentPart::Text { text } if !text.is_empty() => Some(text.clone()),
        _ => None,
    }).collect::<Vec<_>>().join("\n");

    match msg.role.as_str() {
        "user" => Some(Message::User {
            content,
            attachments: Vec::new(),
        }),
        "assistant" => {
            let tool_calls = msg.content.iter().filter_map(|part| match part {
                ContentPart::ToolUse { id, name, input } => Some(CoreToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                }),
                _ => None,
            }).collect();

            Some(Message::Assistant {
                content,
                tool_calls,
                thinking: None,
            })
        }
        "tool" => {
            let tool_use_id = msg.content.iter().find_map(|part| {
                if let ContentPart::ToolResult { tool_use_id, .. } = part {
                    Some(tool_use_id.clone())
                } else {
                    None
                }
            }).unwrap_or_else(|| {
                tracing::warn!("Tool result missing tool_use_id");
                "unknown".to_string()
            });

            let content = msg.content.iter().filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            }).collect::<Vec<_>>().join(" ");

            Some(Message::ToolResult {
                tool_call_id: tool_use_id,
                content,
                is_error: false,
            })
        }
        _ => None,
    }
}

/// Execute a tool call (rig variant).
pub(crate) async fn execute_tool_rig(
    id: String,
    name: String,
    input: serde_json::Value,
    hooks: &[Arc<dyn Hook>],
    registry: &ToolRegistry,
    _event_tx: &mpsc::Sender<AgentEvent>,
) -> ToolResult {
    let tool_call = CoreToolCall {
        id: id.clone(),
        name: name.clone(),
        arguments: input.clone(),
    };
    let ctx = Context::default();

    // Run before hooks
    let (current_args, blocked, block_reason) = run_before_hooks_rig(hooks, &tool_call, &input, &ctx).await;

    if blocked {
        return ToolResult {
            tool_call_id: id,
            tool_name: name,
            input,
            content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
            is_error: true,
        };
    }

    execute_tool_internal_rig(id, name, current_args, &tool_call, hooks, &ctx, registry).await
}

async fn run_before_hooks_rig(
    hooks: &[Arc<dyn Hook>],
    tool_call: &CoreToolCall,
    input: &serde_json::Value,
    ctx: &Context,
) -> (serde_json::Value, bool, String) {
    let mut current_args = input.clone();
    let mut blocked = false;
    let mut block_reason = String::new();

    for hook in hooks {
        match hook.before_tool_call(&CoreToolCall { arguments: current_args.clone(), ..tool_call.clone() }, ctx).await {
            Ok(HookDecision::Allow) => {}
            Ok(HookDecision::Block { reason }) => {
                blocked = true;
                block_reason = reason;
                break;
            }
            Ok(HookDecision::Modify { args }) => current_args = args,
            Err(e) => {
                eprintln!("Hook error: {}", e);
                blocked = true;
                block_reason = format!("Hook error: {}", e);
                break;
            }
        }
    }

    (current_args, blocked, block_reason)
}

async fn execute_tool_internal_rig(
    id: String,
    name: String,
    final_input: serde_json::Value,
    tool_call: &CoreToolCall,
    hooks: &[Arc<dyn Hook>],
    ctx: &Context,
    registry: &ToolRegistry,
) -> ToolResult {
    if let Some(tool) = registry.get(&name) {
        match tool.execute(final_input.clone()).await {
            Ok(output) => {
                let final_output = run_after_hooks_rig(hooks, tool_call, output, ctx).await;
                ToolResult {
                    tool_call_id: id,
                    tool_name: name,
                    input: final_input,
                    content: vec![ContentPart::Text { text: final_output.content }],
                    is_error: final_output.terminate,
                }
            }
            Err(e) => ToolResult {
                tool_call_id: id.clone(),
                tool_name: name.clone(),
                input: final_input,
                content: vec![ContentPart::Text { text: e.to_string() }],
                is_error: true,
            },
        }
    } else {
        ToolResult {
            tool_call_id: id.clone(),
            tool_name: name.clone(),
            input: final_input,
            content: vec![ContentPart::Text { text: format!("Tool '{}' not found", name) }],
            is_error: true,
        }
    }
}

async fn run_after_hooks_rig(
    hooks: &[Arc<dyn Hook>],
    tool_call: &CoreToolCall,
    mut output: runie_core::ToolOutput,
    ctx: &Context,
) -> runie_core::ToolOutput {
    for hook in hooks {
        match hook.after_tool_call(tool_call, &output, ctx).await {
            Ok(processed) => output = processed,
            Err(e) => {
                eprintln!("After-hook error: {}", e);
                output = runie_core::ToolOutput {
                    content: format!("After-hook error: {}", e),
                    metadata: serde_json::Value::Null,
                    terminate: true,
                };
            }
        }
    }
    output
}
