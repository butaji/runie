//! Rig-based agent loop using rig-core's streaming API.
//!
//! This module provides an alternative to `loop_engine.rs` that uses rig's
//! streaming completion API with macro-based dispatch to avoid generic type issues.

use crate::events::{AgentEvent, AgentMessage, ContentPart, ToolResult};
use crate::permission::PermissionGate;
use crate::{Hook, HookDecision};
use futures::StreamExt;
use runie_ai::RigProvider;
use runie_core::{Message, ToolCall as CoreToolCall, Context, ToolSchema};
use runie_tools::ToolRegistry;
use rig_core::client::CompletionClient;
use rig_core::completion::CompletionModel;
use rig_core::streaming::StreamedAssistantContent;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum AgentLoopError {
    ProviderError(String),
    ToolError(String),
    SendError(String),
    MaxTurnsExceeded,
    RigError(String),
}

impl std::fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoopError::ProviderError(s) => write!(f, "Provider error: {}", s),
            AgentLoopError::ToolError(s) => write!(f, "Tool error: {}", s),
            AgentLoopError::SendError(s) => write!(f, "Send error: {}", s),
            AgentLoopError::MaxTurnsExceeded => write!(f, "Max turns exceeded"),
            AgentLoopError::RigError(s) => write!(f, "Rig error: {}", s),
        }
    }
}

impl std::error::Error for AgentLoopError {}

/// Converts our Message type to rig's message format.
fn convert_message_to_rig(msg: &Message) -> rig_core::completion::Message {
    use rig_core::completion::message::{AssistantContent, ToolCall, ToolFunction, UserContent};
    use rig_core::OneOrMany;
    
    match msg {
        Message::System { content } => rig_core::completion::Message::System {
            content: content.clone(),
        },
        Message::User { content, attachments: _ } => rig_core::completion::Message::User {
            content: OneOrMany::one(UserContent::Text(rig_core::completion::message::Text {
                text: content.clone(),
            })),
        },
        Message::Assistant { content, tool_calls, thinking: _ } => {
            let mut contents = Vec::new();
            if !content.is_empty() {
                contents.push(AssistantContent::Text(rig_core::completion::message::Text {
                    text: content.clone(),
                }));
            }
            for tc in tool_calls {
                contents.push(AssistantContent::ToolCall(ToolCall {
                    id: tc.id.clone(),
                    call_id: None,
                    function: ToolFunction {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    },
                    signature: None,
                    additional_params: None,
                }));
            }
            if contents.is_empty() {
                contents.push(AssistantContent::Text(rig_core::completion::message::Text {
                    text: String::new(),
                }));
            }
            let content = OneOrMany::many(contents).unwrap_or_else(|_|
                OneOrMany::one(AssistantContent::Text(rig_core::completion::message::Text {
                    text: String::new(),
                }))
            );
            rig_core::completion::Message::Assistant { id: None, content }
        }
        Message::ToolResult { tool_call_id, content, is_error: _ } => {
            rig_core::completion::Message::User {
                content: OneOrMany::one(UserContent::ToolResult(rig_core::completion::message::ToolResult {
                    id: tool_call_id.clone(),
                    call_id: None,
                    content: OneOrMany::one(rig_core::completion::message::ToolResultContent::Text(
                        rig_core::completion::message::Text { text: content.clone() }
                    )),
                }))
            }
        }
    }
}

/// Converts our tool schemas to rig tool definitions.
fn convert_tools_to_rig(tools: &[ToolSchema]) -> Vec<rig_core::completion::request::ToolDefinition> {
    tools.iter().map(|t| {
        rig_core::completion::request::ToolDefinition {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        }
    }).collect()
}

/// Runs the agent loop using rig's streaming API with macro-based dispatch.
pub async fn run_rig_agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: crate::loop_engine::AgentLoopConfig,
    provider: &RigProvider,
    _tool_schemas: Vec<ToolSchema>,
    event_tx: mpsc::Sender<AgentEvent>,
    permission_gate: PermissionGate,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<(), AgentLoopError> {
    // Convert initial messages to rig format
    let system_prompt = config.system_prompt.clone();
    let max_turns = config.max_turns;
    
    // Build rig messages from initial messages
    let mut rig_messages: Vec<rig_core::completion::Message> = vec![
        rig_core::completion::Message::System { content: system_prompt }
    ];
    
    for msg in &initial_messages {
        if let Some(core_msg) = agent_message_to_core(msg) {
            rig_messages.push(convert_message_to_rig(&core_msg));
        }
    }
    
    let rig_tools = convert_tools_to_rig(&registry.schemas());
    
    // Use macro to dispatch to concrete provider type and stream
    runie_ai::with_rig_provider!(provider, client, model_name, {
        // Get the model
        let model = client.completion_model(model_name);
        
        // Build completion request
        if rig_messages.is_empty() {
            return Err(AgentLoopError::ProviderError("No messages provided".to_string()));
        }
        
        let prompt = rig_messages.last().cloned().unwrap();
        let chat_history = rig_messages.into_iter().rev().skip(1).rev().collect::<Vec<_>>();
        
        let mut builder = model.completion_request(prompt);
        if !chat_history.is_empty() {
            builder = builder.messages(chat_history);
        }
        if !rig_tools.is_empty() {
            builder = builder.tools(rig_tools);
        }
        
        let request = builder.build();
        
        // Stream completion
        let stream = model.stream(request)
            .await
            .map_err(|e| AgentLoopError::RigError(e.to_string()))?;
        
        // Process stream
        process_stream(
            stream,
            event_tx,
            permission_gate,
            registry,
            hooks,
            max_turns,
        ).await
    })
}

/// Process the streaming response and handle tool execution.
async fn process_stream<R>(
    mut stream: rig_core::streaming::StreamingCompletionResponse<R>,
    event_tx: mpsc::Sender<AgentEvent>,
    mut permission_gate: PermissionGate,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
    _max_turns: usize,
) -> Result<(), AgentLoopError>
where
    R: Clone + Unpin + rig_core::completion::GetTokenUsage + Send + 'static,
{
    let mut assistant_message = AgentMessage {
        role: "assistant".to_string(),
        content: vec![ContentPart::Text { text: String::new() }],
        timestamp: chrono::Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
    };
    
    event_tx.send(AgentEvent::MessageStart { message: assistant_message.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    
    let mut pending_tool_calls: Vec<(String, String, serde_json::Value)> = vec![];
    let mut text_content = String::new();
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(StreamedAssistantContent::Text(text)) => {
                text_content.push_str(&text.text);
                assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                event_tx.send(AgentEvent::MessageUpdate { message: assistant_message.clone() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            }
            Ok(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                let tool_name = tool_call.function.name.clone();
                let tool_args = tool_call.function.arguments.clone();
                let tool_id = Uuid::new_v4().to_string();
                pending_tool_calls.push((
                    tool_id.clone(),
                    tool_name.clone(),
                    tool_args.clone(),
                ));
                assistant_message.content.push(ContentPart::ToolUse {
                    id: tool_id,
                    name: tool_name,
                    input: tool_args,
                });
                event_tx.send(AgentEvent::MessageUpdate { message: assistant_message.clone() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            }
            Ok(StreamedAssistantContent::ToolCallDelta { id: _, content, .. }) => {
                match content {
                    rig_core::streaming::ToolCallDeltaContent::Name(_name) => {
                        // Name delta - we already have the name from ToolCall
                    }
                    rig_core::streaming::ToolCallDeltaContent::Delta(delta) => {
                        // Arguments delta - merge with existing args
                        if let Some((_, _, args)) = pending_tool_calls.last_mut() {
                            // args is already a serde_json::Value, merge the delta string into it
                            if let Some(obj) = args.as_object_mut() {
                                if let Ok(delta_val) = serde_json::from_str::<serde_json::Value>(&delta) {
                                    if let Some(delta_obj) = delta_val.as_object() {
                                        for (k, v) in delta_obj {
                                            obj.insert(k.clone(), v.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(StreamedAssistantContent::Reasoning(r)) => {
                // Handle reasoning/thinking - send as message update
                let thinking_msg = AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![ContentPart::Text { text: format!("[thinking: {}]", r.display_text()) }],
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                };
                event_tx.send(AgentEvent::MessageUpdate { message: thinking_msg }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            }
            Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => {
                let thinking_msg = AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![ContentPart::Text { text: format!("[thinking: {}]", reasoning) }],
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                };
                event_tx.send(AgentEvent::MessageUpdate { message: thinking_msg }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            }
            Ok(_) => {
                // Ignore other stream items
            }
            Err(e) => {
                assistant_message.error_message = Some(e.to_string());
                event_tx.send(AgentEvent::Error { message: e.to_string() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
                break;
            }
        }
    }
    
    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
    event_tx.send(AgentEvent::MessageEnd { message: assistant_message.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    
    // No tool calls - turn done
    if pending_tool_calls.is_empty() {
        event_tx.send(AgentEvent::TurnEnd { message: assistant_message.clone(), tool_results: vec![] }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
        event_tx.send(AgentEvent::AgentEnd { messages: vec![assistant_message] }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
        return Ok(());
    }
    
    // Execute tool calls
    let mut tool_results = vec![];
    for (tool_id, tool_name, args) in pending_tool_calls {
        event_tx.send(AgentEvent::ToolExecutionStart { tool_call_id: tool_id.clone() }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
        
        event_tx.send(AgentEvent::PermissionRequest {
            tool_call_id: tool_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: serde_json::to_string(&args).unwrap_or_default(),
        }).await.map_err(|e| AgentLoopError::SendError(e.to_string()))?;
        
        use crate::permission::PermissionResult;
        let should_execute = match permission_gate.request_permission(&tool_name, &tool_id).await {
            PermissionResult::Allowed => {
                event_tx.send(AgentEvent::PermissionGranted { tool_call_id: tool_id.clone() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
                true
            }
            PermissionResult::Skipped | PermissionResult::Denied => {
                event_tx.send(AgentEvent::PermissionDenied { tool_call_id: tool_id.clone() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
                false
            }
        };
        
        let result = if !should_execute {
            ToolResult {
                tool_call_id: tool_id.clone(),
                tool_name: tool_name.clone(),
                input: args.clone(),
                content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
                is_error: true,
            }
        } else {
            execute_tool(
                tool_id.clone(),
                tool_name.clone(),
                args.clone(),
                &hooks,
                &registry,
                &event_tx,
            ).await
        };
        
        event_tx.send(AgentEvent::ToolExecutionEnd { tool_call_id: tool_id.clone(), result: result.clone() }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
        
        tool_results.push(result);
    }
    
    event_tx.send(AgentEvent::TurnEnd { message: assistant_message.clone(), tool_results }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    
    event_tx.send(AgentEvent::AgentEnd { messages: vec![assistant_message] }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    
    Ok(())
}

/// Converts AgentMessage to runie_core::Message.
fn agent_message_to_core(msg: &AgentMessage) -> Option<Message> {
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

/// Execute a tool call.
async fn execute_tool(
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
    let mut current_args = input.clone();
    let mut blocked = false;
    let mut block_reason = String::new();

    for hook in hooks {
        match hook.before_tool_call(&CoreToolCall { arguments: current_args.clone(), ..tool_call.clone() }, &ctx).await {
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

    if blocked {
        return ToolResult {
            tool_call_id: id,
            tool_name: name,
            input,
            content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
            is_error: true,
        };
    }

    // Execute tool
    let final_input = current_args.clone();
    if let Some(tool) = registry.get(&name) {
        match tool.execute(final_input.clone()).await {
            Ok(output) => {
                // Run after hooks
                let mut final_output = output;
                for hook in hooks {
                    match hook.after_tool_call(&tool_call, &final_output, &ctx).await {
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
                return ToolResult {
                    tool_call_id: id,
                    tool_name: name,
                    input: final_input,
                    content: vec![ContentPart::Text { text: final_output.content }],
                    is_error: final_output.terminate,
                };
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
