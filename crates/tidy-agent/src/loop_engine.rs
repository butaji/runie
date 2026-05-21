use crate::events::*;
use crate::pi::AgentTool;
use tidy_ai::Provider;
use tidy_core::{Message, ToolSchema, Event as LlmEvent};
use tokio::sync::mpsc;
use futures::StreamExt;
use chrono::Utc;

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

                // Find and execute tool
                let result = if let Some(tool) = tools.iter().find(|t| t.name == *name) {
                    if let Some(ref handler) = tool.handler {
                        match handler(input.clone()) {
                            Ok(output) => ToolResult {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                input: input.clone(),
                                content: vec![ContentPart::Text { text: output }],
                                is_error: false,
                            },
                            Err(err) => ToolResult {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                input: input.clone(),
                                content: vec![ContentPart::Text { text: err }],
                                is_error: true,
                            },
                        }
                    } else {
                        ToolResult {
                            tool_call_id: id.clone(),
                            tool_name: name.clone(),
                            input: input.clone(),
                            content: vec![ContentPart::Text { text: "Tool has no handler".to_string() }],
                            is_error: true,
                        }
                    }
                } else {
                    ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
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
    let mut llm_msgs = vec![
        Message::System { content: system_prompt.to_string() },
    ];

    for msg in messages {
        let content = msg.content.iter().map(|part| match part {
            ContentPart::Text { text } => text.clone(),
            ContentPart::ToolUse { name, input, .. } => format!("{}({})", name, input),
            ContentPart::ToolResult { content, .. } => content.iter().map(|c| match c {
                ContentPart::Text { text } => text.clone(),
                _ => String::new(),
            }).collect::<Vec<_>>().join(" "),
            _ => String::new(),
        }).collect::<Vec<_>>().join("\n");

        let role = match msg.role.as_str() {
            "user" => Some(Message::User { content, attachments: Vec::new() }),
            "assistant" => Some(Message::Assistant {
                content,
                tool_calls: Vec::new(),
                thinking: None,
            }),
            "tool" => {
                // Extract tool_call_id from ToolResult if present
                let tool_call_id = msg.content.iter().find_map(|part| {
                    if let ContentPart::ToolResult { tool_use_id, .. } = part {
                        Some(tool_use_id.clone())
                    } else {
                        None
                    }
                }).unwrap_or_else(|| "unknown".to_string());
                Some(Message::ToolResult {
                    tool_call_id,
                    content,
                    is_error: false,
                })
            }
            _ => None,
        };

        if let Some(msg) = role {
            llm_msgs.push(msg);
        }
    }

    llm_msgs
}
