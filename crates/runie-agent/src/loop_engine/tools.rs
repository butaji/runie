use crate::events::*;
use crate::{Hook, HookDecision, ToolResult};
use crate::tools::AgentTool;
use super::permissions::{request_permission, add_denied_result, add_blocked_result, add_tool_result};
use super::streaming::PartialToolCall;
use runie_core::{Context, ToolCall as CoreToolCall};
use runie_tools::ToolRegistry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// P1-3 FIX: Execute tool with panic recovery
/// Wraps tool execution in a catch_unwind to prevent panics from crashing the agent.
/// Returns Ok(result) on success, Err(panic_message) if tool panicked.
///
/// NOTE: This currently catches panics from data preparation only, not from the
/// async tool.execute() call itself. Async panics would need a different isolation
/// mechanism (e.g., a dedicated worker process/thread) to catch properly.
pub(crate) async fn execute_tool_with_panic_catch(
    registry: Arc<ToolRegistry>,
    name: &str,
    input: serde_json::Value,
    hooks: Vec<Arc<dyn Hook>>,
    tool_call: CoreToolCall,
    ctx: Context,
) -> Result<ToolResult, String> {
    // Run prep phase with panic catching
    let prep_data = run_prep_phase(registry.clone(), name, input.clone(), hooks.clone(), tool_call.clone(), ctx.clone()).await?;

    // Execute tool with after hooks
    execute_tool_core(prep_data).await
}

struct PrepData {
    name_str: String,
    input_final: serde_json::Value,
    registry_final: Arc<ToolRegistry>,
    hooks_final: Vec<Arc<dyn Hook>>,
    tool_call_final: CoreToolCall,
    ctx_final: Context,
}

async fn run_prep_phase(
    registry: Arc<ToolRegistry>,
    name: &str,
    input: serde_json::Value,
    hooks: Vec<Arc<dyn Hook>>,
    tool_call: CoreToolCall,
    ctx: Context,
) -> Result<PrepData, String> {
    let registry_clone = registry.clone();
    let name_clone = name.to_string();
    let input_clone = input.clone();
    let hooks_clone = hooks.clone();
    let tool_call_clone = tool_call.clone();
    let ctx_clone = ctx.clone();

    let prep_result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            (name_clone, input_clone, registry_clone, hooks_clone, tool_call_clone, ctx_clone)
        }))
    }).await;

    match prep_result {
        Ok(Ok((name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final))) => {
            Ok(PrepData { name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final })
        }
        Ok(Err(panic_info)) => Err(extract_panic_message(panic_info)),
        Err(join_err) => Err(format!("Task execution failed: {}", join_err)),
    }
}

async fn execute_tool_core(prep_data: PrepData) -> Result<ToolResult, String> {
    let PrepData { name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final } = prep_data;

    // Execute the tool (async - panics here cannot be caught by catch_unwind)
    let output_result = if let Some(tool) = registry_final.get(&name_str) {
        tool.execute(input_final.clone()).await
    } else {
        return Ok(ToolResult {
            tool_call_id: tool_call_final.id.clone(),
            tool_name: name_str.clone(),
            input: input_final,
            content: vec![ContentPart::Text { text: format!("Tool '{}' not found", name_str) }],
            is_error: true,
        });
    };

    // Run after hooks
    let final_output = process_after_hooks(output_result, &hooks_final, &tool_call_final, &ctx_final).await;

    Ok(ToolResult {
        tool_call_id: tool_call_final.id.clone(),
        tool_name: name_str.clone(),
        input: input_final,
        content: vec![ContentPart::Text { text: final_output.content }],
        is_error: final_output.terminate,
    })
}

async fn process_after_hooks(
    output_result: Result<runie_core::ToolOutput, runie_core::ToolError>,
    hooks: &[Arc<dyn Hook>],
    tool_call: &CoreToolCall,
    ctx: &Context,
) -> runie_core::ToolOutput {
    match output_result {
        Ok(mut output) => {
            for hook in hooks {
                match hook.after_tool_call(tool_call, &output, ctx).await {
                    Ok(processed) => output = processed,
                    Err(e) => {
                        tracing::error!("After-hook error: {}", e);
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
        Err(e) => {
            runie_core::ToolOutput {
                content: e.to_string(),
                metadata: serde_json::Value::Null,
                terminate: true,
            }
        }
    }
}

/// Extract panic message from panic payload
pub(crate) fn extract_panic_message(panic_info: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

/// Run before hooks for a tool call. Returns (modified_args, blocked, block_reason).
pub(crate) async fn run_before_hooks(
    hooks: &[Arc<dyn Hook>],
    tool_call: &CoreToolCall,
    input: serde_json::Value,
    ctx: &Context,
) -> (serde_json::Value, bool, String) {
    let mut current_args = input;
    let mut blocked = false;
    let mut block_reason = String::new();

    for hook in hooks {
        match hook.before_tool_call(&CoreToolCall { arguments: current_args.clone(), ..tool_call.clone() }, ctx).await {
            Ok(HookDecision::Allow) => {},
            Ok(HookDecision::Block { reason }) => {
                blocked = true;
                block_reason = reason;
                break;
            }
            Ok(HookDecision::Modify { args }) => {
                current_args = args;
            }
            Err(e) => {
                tracing::error!("Hook error: {}", e);
                blocked = true;
                block_reason = format!("Hook error: {}", e);
                break;
            }
        }
    }

    (current_args, blocked, block_reason)
}

/// Process tool calls - validate, request permissions, and execute tools.
pub(crate) async fn process_tool_calls<M: TryFrom<AgentEvent> + Send + 'static>(
    messages: &mut Vec<AgentMessage>,
    mut pending_tool_calls: HashMap<(String, String), PartialToolCall>,
    turn_count: usize,
    tools: &[AgentTool],
    allowed_tools: &HashSet<String>,
    msg_tx: &tokio::sync::mpsc::Sender<M>,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
    _allowed_tools_mut: &mut HashSet<String>,
    context_window: usize,
) -> Vec<ToolResult>
where
    M: TryFrom<AgentEvent> + Send + 'static,
{
    let mut tool_results = vec![];
    let mut seen_tool_calls: HashSet<String> = HashSet::new();

    tracing::info!("[ACTOR:AgentLoop] {} tool calls finalized", pending_tool_calls.len());
    for partial in pending_tool_calls.values() {
        tracing::info!("[ACTOR:AgentLoop] id={} name={} accumulated_args={:?}", partial.id, partial.name, partial.arguments);
    }

    let finalized_calls: Vec<(ContentPart, String, String)> = pending_tool_calls.drain().map(|((id, name), partial)| {
        let input = match serde_json::from_str(&partial.arguments) {
            Ok(v) => v,
            Err(_) => serde_json::json!({"raw": partial.arguments}),
        };
        (ContentPart::ToolUse { id: id.clone(), name: name.clone(), input }, name.clone(), partial.arguments)
    }).collect();

    for (tool_use, _tool_name, _args_str) in finalized_calls {
        if let ContentPart::ToolUse { id, name, input } = &tool_use {
            // Validate tool name
            if name.trim().is_empty() {
                tracing::warn!("Tool call with empty name skipped (call_id: {})", id);
                super::streaming::send_event(msg_tx, AgentEvent::Error {
                    message: format!("Tool call '{}' has empty name - skipping", id),
                    error_type: "invalid_tool_call".to_string(),
                    recoverable: true,
                    context: format!("The model generated a tool call without a name. Raw input: {:?}", input),
                }).await;
                continue;
            }

            // Validate tool exists
            if !tools.iter().any(|t| t.name == *name) {
                tracing::warn!("Tool '{}' not found in registry (call_id: {})", name, id);
                super::streaming::send_event(msg_tx, AgentEvent::Error {
                    message: format!("Tool '{}' not found", name),
                    error_type: "tool_not_found".to_string(),
                    recoverable: true,
                    context: format!("Available tools: {}", tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ")),
                }).await;
                continue;
            }

            // Check for duplicates
            let tool_key = format!("{}:{}", name, serde_json::to_string(input).unwrap_or_default());
            if seen_tool_calls.contains(&tool_key) {
                tracing::warn!("Duplicate tool call detected and skipped: {} with args {:?}", name, input);
                continue;
            }
            seen_tool_calls.insert(tool_key);

            let tool_args = serde_json::to_string(input).unwrap_or_default();
            let context_window_usage = super::calculate_context_window_usage(messages, context_window);

            super::streaming::send_event(msg_tx, AgentEvent::ToolExecutionStart {
                tool_call_id: id.clone(),
                tool_name: name.clone(),
                tool_args: tool_args.clone(),
                turn: turn_count,
            }).await;
            tracing::info!("[ACTOR:AgentLoop] {} requested: {}", name, tool_args);

            // Check permission
            let should_execute = if allowed_tools.contains(name) {
                true
            } else {
                let tool_description = tools.iter()
                    .find(|t| t.name == *name)
                    .map(|t| t.description.clone())
                    .unwrap_or_default();

                request_permission(
                    &id,
                    &name,
                    &tool_args,
                    tool_description,
                    context_window_usage,
                    turn_count,
                    permission_state.clone(),
                    msg_tx,
                ).await
            };

            if !should_execute {
                add_denied_result(messages, &id, &name, input.clone());
                let result = ToolResult {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    input: input.clone(),
                    content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
                    is_error: true,
                };
                tool_results.push(result);
                continue;
            }

            // Execute tool
            let start_time = Instant::now();
            let tool_call = CoreToolCall {
                id: id.clone(),
                name: name.clone(),
                arguments: input.clone(),
            };
            let ctx = Context::default();

            // Run before hooks
            let (final_input, blocked, block_reason) = run_before_hooks(&hooks, &tool_call, input.clone(), &ctx).await;

            if blocked {
                let blocked_result = ToolResult {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    input: input.clone(),
                    content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
                    is_error: true,
                };
                let duration_ms = start_time.elapsed().as_millis() as u64;
                super::streaming::send_event(msg_tx, AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: tool_args.clone(),
                    result: blocked_result.clone(),
                    duration_ms,
                    turn: turn_count,
                }).await;
                add_blocked_result(messages, &id, &name, input.clone(), &block_reason);
                tool_results.push(blocked_result);
                continue;
            }

            // Execute with panic catch
            let tool_execution = execute_tool_with_panic_catch(
                registry.clone(),
                &name,
                final_input.clone(),
                hooks.clone(),
                tool_call.clone(),
                ctx.clone(),
            ).await;

            let result = match tool_execution {
                Ok(result) => result,
                Err(panic_msg) => {
                    tracing::error!("Tool '{}' panicked: {}", name, panic_msg);
                    let panic_result = ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: final_input,
                        content: vec![ContentPart::Text {
                            text: format!("Tool '{}' panicked (internal error). State has been rolled back.", name)
                        }],
                        is_error: true,
                    };
                super::streaming::send_event(msg_tx, AgentEvent::Error {
                        message: format!("Tool '{}' panicked: {}", name, panic_msg),
                        error_type: "tool_panic".to_string(),
                        recoverable: true,
                        context: format!("Tool '{}' panicked during execution", name),
                    }).await;
                    panic_result
                }
            };

            let duration_ms = start_time.elapsed().as_millis() as u64;
            super::streaming::send_event(msg_tx, AgentEvent::ToolExecutionEnd {
                tool_call_id: id.clone(),
                tool_name: name.clone(),
                tool_args: tool_args.clone(),
                result: result.clone(),
                duration_ms,
                turn: turn_count,
            }).await;

            let result_preview = result.content.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join("; ");
            tracing::info!("[ACTOR:AgentLoop] {} result: {} ({}ms)", name, result_preview.chars().take(100).collect::<String>(), duration_ms);

            add_tool_result(messages, &id, &result);
            tool_results.push(result);
        }
    }

    tool_results
}
