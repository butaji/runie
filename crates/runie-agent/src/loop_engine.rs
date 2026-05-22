use crate::events::*;
use crate::pi::AgentTool;
use crate::{Hook, HookDecision};
use runie_ai::Provider;
use runie_core::{Message, ToolSchema, Event as LlmEvent, Context};
use runie_tools::ToolRegistry;
use tokio::sync::mpsc;
use futures::StreamExt;
use chrono::Utc;
use std::sync::Arc;

pub struct AgentLoopConfig {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: String,
}

pub async fn run_agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: &dyn Provider,
    tools: &[AgentTool],
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    mut permission_rx: mpsc::UnboundedReceiver<PermissionDecision>,
    registry: Option<Arc<ToolRegistry>>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<(), String> {
    let mut messages = initial_messages;

    // Convert tools to schema
    let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| ToolSchema {
        name: t.name.clone(),
        description: t.description.clone(),
        parameters: t.parameters.clone(),
    }).collect();

    loop {
        // Build LLM messages
        let llm_messages = build_llm_messages(&config.system_prompt, &messages);

        // Start streaming
        let stream = provider.chat(llm_messages, tool_schemas.clone()).await
            .map_err(|e| e.to_string())?;

        // Send message_start event
        let mut assistant_message = AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: String::new() }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
        };

        event_tx.send(AgentEvent::MessageStart {
            message: assistant_message.clone(),
        }).ok();

        // Process stream
        let mut pending_tool_calls: Vec<(ContentPart, String, String)> = vec![];
        let mut text_content = String::new();

        let mut stream = stream;
        while let Some(event) = stream.next().await {
            match event {
                LlmEvent::MessageDelta { content } => {
                    text_content.push_str(&content);
                    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                    event_tx.send(AgentEvent::MessageUpdate {
                        message: assistant_message.clone(),
                    }).ok();
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
                    event_tx.send(AgentEvent::TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    }).ok();
                }
                _ => {}
            }
        }

        // Send message_end
        assistant_message.content = vec![ContentPart::Text { text: text_content }];
        event_tx.send(AgentEvent::MessageEnd {
            message: assistant_message.clone(),
        }).ok();

        messages.push(assistant_message.clone());

        // Execute tool calls
        if pending_tool_calls.is_empty() {
            // No tools, turn is done
            event_tx.send(AgentEvent::TurnEnd {
                message: assistant_message.clone(),
                tool_results: vec![],
            }).ok();
            break;
        }

        let mut tool_results = vec![];
        for (tool_use, _tool_name, _args_str) in pending_tool_calls {
            if let ContentPart::ToolUse { id, name, input } = &tool_use {
                event_tx.send(AgentEvent::ToolExecutionStart {
                    tool_call_id: id.clone(),
                }).ok();

                // Send permission request
                event_tx.send(AgentEvent::PermissionRequest {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: serde_json::to_string(input).unwrap_or_default(),
                }).ok();

                // Wait for permission decision
                let decision = tokio::time::timeout(
                    std::time::Duration::from_secs(300), // 5 minute timeout
                    permission_rx.recv()
                ).await;

                let should_execute = match decision {
                    Ok(Some(PermissionDecision::Allow)) | Ok(Some(PermissionDecision::AllowAlways)) => {
                        event_tx.send(AgentEvent::PermissionGranted {
                            tool_call_id: id.clone(),
                        }).ok();
                        true
                    }
                    Ok(Some(PermissionDecision::Skip)) => {
                        event_tx.send(AgentEvent::PermissionDenied {
                            tool_call_id: id.clone(),
                        }).ok();
                        false // Skip this tool but continue with others
                    }
                    _ => {
                        // Timeout or deny
                        event_tx.send(AgentEvent::PermissionDenied {
                            tool_call_id: id.clone(),
                        }).ok();
                        false
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

                    event_tx.send(AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        result: result.clone(),
                    }).ok();

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
                let args_to_use = 'hook_block: {
                    for hook in &hooks {
                        match hook.before_tool_call(&tool_call, &ctx).await {
                            Ok(HookDecision::Allow) => break 'hook_block input.clone(),
                            Ok(HookDecision::Block { reason }) => {
                                let blocked_result = ToolResult {
                                    tool_call_id: id.clone(),
                                    tool_name: name.clone(),
                                    input: input.clone(),
                                    content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", reason) }],
                                    is_error: true,
                                };
                                event_tx.send(AgentEvent::ToolExecutionEnd {
                                    tool_call_id: id.clone(),
                                    result: blocked_result.clone(),
                                }).ok();
                                tool_results.push(blocked_result);
                                continue;
                            }
                            Ok(HookDecision::Modify { args }) => break 'hook_block args,
                            Err(_) => {}
                        }
                    }
                    input.clone()
                };
                
                let final_input = args_to_use.clone();
                let result = if let Some(ref reg) = registry {
                    if let Some(tool) = reg.get(name) {
                        match tool.execute(final_input.clone()).await {
                            Ok(output) => {
                                // Run after hooks
                                let final_output = if let Some(hook) = hooks.first() {
                                    match hook.after_tool_call(&tool_call, &output, &ctx).await {
                                        Ok(processed) => processed,
                                        Err(_) => output,
                                    }
                                } else {
                                    output
                                };
                                
                                ToolResult {
                                    tool_call_id: id.clone(),
                                    tool_name: name.clone(),
                                    input: final_input,
                                    content: vec![ContentPart::Text { text: final_output.content }],
                                    is_error: false,
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
                } else if let Some(tool) = tools.iter().find(|t| t.name == *name) {
                    if let Some(ref handler) = tool.handler {
                        match handler(final_input.clone()) {
                            Ok(output) => ToolResult {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                input: final_input,
                                content: vec![ContentPart::Text { text: output }],
                                is_error: false,
                            },
                            Err(err) => ToolResult {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                input: final_input,
                                content: vec![ContentPart::Text { text: err }],
                                is_error: true,
                            },
                        }
                    } else {
                        ToolResult {
                            tool_call_id: id.clone(),
                            tool_name: name.clone(),
                            input: final_input,
                            content: vec![ContentPart::Text { text: "Tool has no handler".to_string() }],
                            is_error: true,
                        }
                    }
                } else {
                    ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: final_input,
                        content: vec![ContentPart::Text { text: format!("Tool '{}' not found", name) }],
                        is_error: true,
                    }
                };

                event_tx.send(AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.clone(),
                    result: result.clone(),
                }).ok();

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

        event_tx.send(AgentEvent::TurnEnd {
            message: assistant_message.clone(),
            tool_results,
        }).ok();

        // Continue loop - send updated messages back to LLM
    }

    event_tx.send(AgentEvent::AgentEnd { messages: messages.clone() }).ok();
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
            }).unwrap_or_else(|| "unknown".to_string());
            Some(Message::ToolResult { tool_call_id, content, is_error: false })
        }
        _ => None,
    }
}
