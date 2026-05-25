use crate::config::AgentConfig;
use crate::events::*;
use crate::tools::AgentTool;
use crate::{Hook, HookDecision};
use runie_ai::Provider;
use runie_core::{Message, ToolSchema, Event as LlmEvent, Context, ToolCall as CoreToolCall};
use runie_tools::ToolRegistry;
use tokio::sync::mpsc;
use futures::StreamExt;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AgentLoopError {
    ProviderError(String),
    ToolError(String),
    SendError(String),
    MaxTurnsExceeded,
}

impl std::fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoopError::ProviderError(s) => write!(f, "Provider error: {}", s),
            AgentLoopError::ToolError(s) => write!(f, "Tool error: {}", s),
            AgentLoopError::SendError(s) => write!(f, "Send error: {}", s),
            AgentLoopError::MaxTurnsExceeded => write!(f, "Max turns exceeded"),
        }
    }
}

pub struct AgentLoopConfig {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: String,
    pub max_turns: usize,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            model: String::new(),
            thinking_level: String::new(),
            max_turns: AgentConfig::default().max_turns,
        }
    }
}

pub async fn run_agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: &dyn Provider,
    tools: &[AgentTool],
    event_tx: mpsc::Sender<AgentEvent>,
    mut permission_rx: mpsc::Receiver<PermissionDecision>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<(), AgentLoopError> {
    let mut messages = initial_messages;
    let mut allowed_tools: HashSet<String> = HashSet::new();

    // Convert tools to schema
    let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| ToolSchema {
        name: t.name.clone(),
        description: t.description.clone(),
        parameters: t.parameters.clone(),
    }).collect();

    let mut turn_count = 0;

    loop {
        turn_count += 1;
        if turn_count > config.max_turns {
            if let Err(e) = event_tx.send(AgentEvent::Error {
                message: format!("Max turns ({}) exceeded", config.max_turns)
            }).await {
                tracing::error!("Failed to send event: {}", e);
            }
            return Err(AgentLoopError::MaxTurnsExceeded);
        }

        // Build LLM messages
        let llm_messages = build_llm_messages(&config.system_prompt, &messages);

        // Start streaming
        let stream = provider.chat(llm_messages, tool_schemas.clone()).await
            .map_err(|e| AgentLoopError::ProviderError(e.to_string()))?;

        // Send message_start event
        let mut assistant_message = AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: String::new() }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
        };

        if let Err(e) = event_tx.send(AgentEvent::MessageStart {
            message: assistant_message.clone(),
        }).await {
            tracing::error!("Failed to send event: {}", e);
        }

        // Process stream
        let mut pending_tool_calls: Vec<(ContentPart, String, String)> = vec![];
        let mut text_content = String::new();

        let mut stream = stream;
        while let Some(event) = stream.next().await {
            match event {
                LlmEvent::MessageDelta { content } => {
                    text_content.push_str(&content);
                    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                    if let Err(e) = event_tx.send(AgentEvent::MessageUpdate {
                        message: assistant_message.clone(),
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }
                }
                LlmEvent::ToolCallDelta { name, arguments } => {
                    pending_tool_calls.push((
                        ContentPart::ToolUse {
                            id: format!("call_{}", pending_tool_calls.len()),
                            name: name.clone(),
                            input: serde_json::json!(arguments),
                        },
                        name,
                        arguments,
                    ));
                }
                LlmEvent::MessageEnd => {
                    // Finalize any pending tool calls
                    break;
                }
                LlmEvent::Error { message } => {
                    assistant_message.error_message = Some(message);
                    break;
                }
                LlmEvent::Usage { prompt_tokens, completion_tokens, total_tokens } => {
                    if let Err(e) = event_tx.send(AgentEvent::TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unhandled LLM event variant in agent loop");
                }
            }
        }

        // Send message_end
        assistant_message.content = vec![ContentPart::Text { text: text_content }];
        if let Err(e) = event_tx.send(AgentEvent::MessageEnd {
            message: assistant_message.clone(),
        }).await {
            tracing::error!("Failed to send event: {}", e);
        }

        messages.push(assistant_message.clone());

        // Execute tool calls
        if pending_tool_calls.is_empty() {
            // No tools, turn is done
            if let Err(e) = event_tx.send(AgentEvent::TurnEnd {
                message: assistant_message.clone(),
                tool_results: vec![],
            }).await {
                tracing::error!("Failed to send event: {}", e);
            }
            break;
        }

        let mut tool_results = vec![];
        for (tool_use, _tool_name, _args_str) in pending_tool_calls {
            if let ContentPart::ToolUse { id, name, input } = &tool_use {
                if let Err(e) = event_tx.send(AgentEvent::ToolExecutionStart {
                    tool_call_id: id.clone(),
                }).await {
                    tracing::error!("Failed to send event: {}", e);
                }

                // Check if tool is in allowed cache first
                let should_execute = if allowed_tools.contains(name) {
                    if let Err(e) = event_tx.send(AgentEvent::PermissionGranted {
                        tool_call_id: id.clone(),
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }
                    true
                } else {
                    // Send permission request
                    if let Err(e) = event_tx.send(AgentEvent::PermissionRequest {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: serde_json::to_string(input).unwrap_or_default(),
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }

                    // Wait for permission decision (correlated by tool_call_id)
                    let decision = tokio::time::timeout(
                        std::time::Duration::from_secs(300), // 5 minute timeout
                        permission_rx.recv()
                    ).await;

                    match decision {
                        Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid })) if tid == id => {
                            if let Err(e) = event_tx.send(AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                            }).await {
                                tracing::error!("Failed to send event: {}", e);
                            }
                            true
                        }
                        Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid })) if tid == id => {
                            // Cache the tool name for future auto-allow
                            allowed_tools.insert(name.clone());
                            if let Err(e) = event_tx.send(AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                            }).await {
                                tracing::error!("Failed to send event: {}", e);
                            }
                            true
                        }
                        Ok(Some(PermissionDecision::Skip { tool_call_id: ref tid })) if tid == id => {
                            if let Err(e) = event_tx.send(AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                            }).await {
                                tracing::error!("Failed to send event: {}", e);
                            }
                            false // Skip this tool but continue with others
                        }
                        Ok(Some(PermissionDecision::Deny { .. })) => {
                            if let Err(e) = event_tx.send(AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                            }).await {
                                tracing::error!("Failed to send event: {}", e);
                            }
                            false
                        }
                        _ => {
                            // Timeout, mismatch, or deny
                            if let Err(e) = event_tx.send(AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                            }).await {
                                tracing::error!("Failed to send event: {}", e);
                            }
                            false
                        }
                    }
                };

                if !should_execute {
                    // Add a fake error result
                    let result = ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                        content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
                        is_error: true,
                    };

                    if let Err(e) = event_tx.send(AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        result: result.clone(),
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }

                    messages.push(AgentMessage {
                        role: "tool".to_string(),
                        content: vec![ContentPart::ToolResult {
                            tool_use_id: id.clone(),
                            content: result.content.clone(),
                            is_error: result.is_error,
                        }],
                        timestamp: Utc::now().timestamp_millis(),
                        usage: None,
                        stop_reason: None,
                        error_message: None,
                    });

                    tool_results.push(result);
                    continue;
                }

                // Find and execute tool
                let tool_call = runie_core::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                };
                let ctx = Context::default();
                
                // Run before hooks
                let mut current_args = input.clone();
                let mut blocked = false;
                let mut block_reason = String::new();
                for hook in &hooks {
                    match hook.before_tool_call(&CoreToolCall { arguments: current_args.clone(), ..tool_call.clone() }, &ctx).await {
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
                            eprintln!("Hook error: {}", e);
                            blocked = true;
                            block_reason = format!("Hook error: {}", e);
                            break;
                        }
                    }
                }

                if blocked {
                    let blocked_result = ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                        content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
                        is_error: true,
                    };
                    if let Err(e) = event_tx.send(AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        result: blocked_result.clone(),
                    }).await {
                        tracing::error!("Failed to send event: {}", e);
                    }
                    tool_results.push(blocked_result);
                    continue;
                }

                let final_input = current_args.clone();
                
                // P1-3 FIX: Wrap tool execution in panic catch to prevent panics from crashing the agent
                let tool_execution = execute_tool_with_panic_catch(
                    registry.clone(),
                    name,
                    final_input.clone(),
                    hooks.clone(),
                    tool_call.clone(),
                    ctx.clone(),
                ).await;
                
                let result = match tool_execution {
                    Ok(result) => result,
                    Err(panic_msg) => {
                        // Tool panicked - return error result and trigger rollback
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
                        
                        // Send panic event to notify TUI
                        let _ = event_tx.send(AgentEvent::Error {
                            message: format!("Tool '{}' panicked: {}", name, panic_msg),
                        }).await;
                        
                        panic_result
                    }
                };

                if let Err(e) = event_tx.send(AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.clone(),
                    result: result.clone(),
                }).await {
                    tracing::error!("Failed to send event: {}", e);
                }

                // Add tool result to messages
                messages.push(AgentMessage {
                    role: "tool".to_string(),
                    content: vec![ContentPart::ToolResult {
                        tool_use_id: id.clone(),
                        content: result.content.clone(),
                        is_error: result.is_error,
                    }],
                    timestamp: Utc::now().timestamp_millis(),
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                });

                tool_results.push(result);
            }
        }

        if let Err(e) = event_tx.send(AgentEvent::TurnEnd {
            message: assistant_message.clone(),
            tool_results,
        }).await {
            tracing::error!("Failed to send event: {}", e);
        }

        // Continue loop - send updated messages back to LLM
    }

    event_tx.send(AgentEvent::AgentEnd { messages: messages.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    Ok(())
}

fn build_llm_messages(system_prompt: &str, messages: &[AgentMessage]) -> Vec<Message> {
    let mut llm_msgs = vec![Message::System { content: system_prompt.to_string() }];
    for msg in messages {
        let content = format_message_content(&msg.content);
        if let Some(m) = agent_msg_to_llm(&msg.role, content, &msg.content) {
            llm_msgs.push(m);
        }
    }
    llm_msgs
}

/// P1-3 FIX: Execute tool with panic recovery
/// Wraps tool execution in a catch_unwind to prevent panics from crashing the agent.
/// Returns Ok(result) on success, Err(panic_message) if tool panicked.
async fn execute_tool_with_panic_catch(
    registry: Arc<ToolRegistry>,
    name: &str,
    input: serde_json::Value,
    hooks: Vec<Arc<dyn Hook>>,
    tool_call: runie_core::ToolCall,
    ctx: Context,
) -> Result<ToolResult, String> {
    // Use tokio::task::spawn_blocking with AssertUnwindSafe to catch panics
    let registry_clone = registry.clone();
    let name_clone = name.to_string();
    let input_clone = input.clone();
    let hooks_clone = hooks.clone();
    let tool_call_clone = tool_call.clone();
    let ctx_clone = ctx.clone();
    
    // Run in blocking task to catch panics from sync code
    let result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            // Prepare tool execution data (sync part)
            (name_clone, input_clone, registry_clone, hooks_clone, tool_call_clone, ctx_clone)
        }))
    }).await;
    
    // Check if spawn_blocking itself panicked
    let prep_result = match result {
        Ok(Ok(data)) => data,
        Ok(Err(panic_info)) => {
            let panic_msg = extract_panic_message(panic_info);
            return Err(panic_msg);
        }
        Err(join_err) => {
            // Task was cancelled or panicked before completion
            return Err(format!("Task execution failed: {}", join_err));
        }
    };
    
    let (name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final) = prep_result;
    
    // Now execute the async tool
    if let Some(tool) = registry_final.get(&name_str) {
        match tool.execute(input_final.clone()).await {
            Ok(output) => {
                // Run after hooks
                let mut final_output = output;
                for hook in &hooks_final {
                    match hook.after_tool_call(&tool_call_final, &final_output, &ctx_final).await {
                        Ok(processed) => final_output = processed,
                        Err(e) => {
                            eprintln!("After-hook error: {}", e);
                            final_output = runie_core::ToolOutput {
                                content: format!("After-hook error: {}", e),
                                metadata: serde_json::Value::Null,
                                terminate: true,
                            };
                        }
                    }
                }
                
                Ok(ToolResult {
                    tool_call_id: tool_call_final.id.clone(),
                    tool_name: name_str.clone(),
                    input: input_final,
                    content: vec![ContentPart::Text { text: final_output.content }],
                    is_error: final_output.terminate,
                })
            }
            Err(e) => Ok(ToolResult {
                tool_call_id: tool_call_final.id.clone(),
                tool_name: name_str.clone(),
                input: input_final,
                content: vec![ContentPart::Text { text: e.to_string() }],
                is_error: true,
            }),
        }
    } else {
        Ok(ToolResult {
            tool_call_id: tool_call_final.id.clone(),
            tool_name: name_str.clone(),
            input: input_final,
            content: vec![ContentPart::Text { text: format!("Tool '{}' not found", name_str) }],
            is_error: true,
        })
    }
}

/// Extract panic message from panic payload
fn extract_panic_message(panic_info: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

fn format_message_content(parts: &[ContentPart]) -> String {
    parts.iter().map(|part| match part {
        ContentPart::Text { text } => text.clone(),
        ContentPart::ToolUse { name, input, .. } => format!("{}({})", name, input),
        ContentPart::ToolResult { content, .. } => content.iter().map(|c| match c {
            ContentPart::Text { text } => text.clone(),
            _ => String::new(),
        }).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }).collect::<Vec<_>>().join("\n")
}

fn agent_msg_to_llm(role: &str, content: String, parts: &[ContentPart]) -> Option<Message> {
    match role {
        "user" => Some(Message::User { content, attachments: Vec::new() }),
        "assistant" => Some(Message::Assistant {
            content,
            tool_calls: Vec::new(),
            thinking: None,
        }),
        "tool" => {
            let tool_call_id = parts.iter().find_map(|part| {
                if let ContentPart::ToolResult { tool_use_id, .. } = part {
                    Some(tool_use_id.clone())
                } else {
                    None
                }
            }).unwrap_or_else(|| {
                tracing::warn!("Tool result missing tool_use_id, using 'unknown'");
                "unknown".to_string()
            });
            Some(Message::ToolResult { tool_call_id, content, is_error: false })
        }
        _ => None,
    }
}
