//! Stream processing helpers for rig-based agent loop.

use crate::events::{AgentEvent, AgentMessage, ContentPart, ToolResult};
use crate::permission::PermissionGate;
use crate::{Hook, HookDecision};
use crate::rig_loop::{agent_message_to_core, should_request_permission, AgentLoopError};
use futures::StreamExt;
use rig_core::streaming::{StreamedAssistantContent, ToolCallDeltaContent};
use runie_core::{Context, ToolCall as CoreToolCall, ToolSchema};
use runie_tools::ToolRegistry;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Process the streaming response and handle tool execution.
pub(crate) async fn process_stream<R>(
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

    event_tx.send(AgentEvent::MessageStart { message: assistant_message.clone(), turn: 0 }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

    let mut pending_tool_calls: Vec<(String, String, serde_json::Value)> = vec![];
    let mut text_content = String::new();
    let mut thinking_content = String::new();
    let mut is_thinking = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(StreamedAssistantContent::Text(text)) => {
                // Text content ends thinking
                if *is_thinking {
                    event_tx.send(AgentEvent::ThinkingEnd { duration_ms: 0, turn: 0 }).await
                        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
                    *is_thinking = false;
                }
                handle_text_chunk(&text.text, &mut text_content, &mut assistant_message, &event_tx).await?;
            }
            Ok(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                handle_tool_call_chunk(&mut pending_tool_calls, &tool_call, &mut assistant_message, &event_tx).await?;
            }
            Ok(StreamedAssistantContent::ToolCallDelta { id: _, content, .. }) => {
                handle_tool_call_delta(&mut pending_tool_calls, content);
            }
            Ok(StreamedAssistantContent::Reasoning(r)) => {
                handle_reasoning_chunk(r.display_text(), &mut thinking_content, &mut is_thinking, &event_tx, 0).await?;
            }
            Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => {
                handle_reasoning_delta(reasoning, &mut thinking_content, &mut is_thinking, &event_tx, 0).await?;
            }
            Ok(_) => {}
            Err(e) => {
                assistant_message.error_message = Some(e.to_string());
                event_tx.send(AgentEvent::Error { message: e.to_string() }).await
                    .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
                break;
            }
        }
    }
    
    // Finalize thinking if still active
    if is_thinking {
        event_tx.send(AgentEvent::ThinkingEnd { duration_ms: 0, turn: 0 }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    }

    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
    event_tx.send(AgentEvent::MessageEnd { message: assistant_message.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

    if pending_tool_calls.is_empty() {
        return finalize_turn_no_tools(event_tx, assistant_message).await;
    }

    let tool_results = execute_tool_calls_rig(
        pending_tool_calls,
        &mut permission_gate,
        &hooks,
        &registry,
        &event_tx,
    ).await?;

    finalize_turn_with_tools(event_tx, assistant_message, tool_results).await
}

async fn execute_tool_calls_rig(
    mut pending_tool_calls: Vec<(String, String, serde_json::Value)>,
    permission_gate: &mut PermissionGate,
    hooks: &[Arc<dyn Hook>],
    registry: &ToolRegistry,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<Vec<ToolResult>, AgentLoopError> {
    let mut tool_results = vec![];
    let mut seen_tool_calls: HashSet<String> = HashSet::new();

    for (tool_id, tool_name, args) in pending_tool_calls.drain(..) {
        let tool_key = format!("{}:{}", tool_name, serde_json::to_string(&args).unwrap_or_default());
        if seen_tool_calls.contains(&tool_key) {
            tracing::warn!("Duplicate tool call detected and skipped: {} with args {:?}", tool_name, args);
            continue;
        }
        seen_tool_calls.insert(tool_key);

        event_tx.send(AgentEvent::ToolExecutionStart { tool_call_id: tool_id.clone() }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

        event_tx.send(AgentEvent::PermissionRequest {
            tool_call_id: tool_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: serde_json::to_string(&args).unwrap_or_default(),
        }).await.map_err(|e| AgentLoopError::SendError(e.to_string()))?;

        let should_execute = should_request_permission(&tool_name, &tool_id, permission_gate, event_tx).await?;

        let result = if !should_execute {
            ToolResult {
                tool_call_id: tool_id.clone(),
                tool_name: tool_name.clone(),
                input: args.clone(),
                content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
                is_error: true,
            }
        } else {
            crate::rig_loop::tool::execute_tool_rig(
                tool_id.clone(),
                tool_name.clone(),
                args.clone(),
                hooks,
                registry,
                event_tx,
            ).await
        };

        event_tx.send(AgentEvent::ToolExecutionEnd { tool_call_id: tool_id.clone(), result: result.clone() }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

        tool_results.push(result);
    }

    Ok(tool_results)
}

async fn handle_text_chunk(
    text: &str,
    text_content: &mut String,
    assistant_message: &mut AgentMessage,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<(), AgentLoopError> {
    text_content.push_str(text);
    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
    event_tx.send(AgentEvent::MessageUpdate { message: assistant_message.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))
}

async fn handle_tool_call_chunk(
    pending_tool_calls: &mut Vec<(String, String, serde_json::Value)>,
    tool_call: &rig_core::completion::message::ToolCall,
    assistant_message: &mut AgentMessage,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<(), AgentLoopError> {
    let tool_name = tool_call.function.name.clone();
    let tool_args = tool_call.function.arguments.clone();
    let tool_id = Uuid::new_v4().to_string();
    pending_tool_calls.push((tool_id.clone(), tool_name.clone(), tool_args.clone()));
    assistant_message.content.push(ContentPart::ToolUse { id: tool_id, name: tool_name, input: tool_args });
    event_tx.send(AgentEvent::MessageUpdate { message: assistant_message.clone() }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))
}

fn handle_tool_call_delta(
    pending_tool_calls: &mut Vec<(String, String, serde_json::Value)>,
    content: ToolCallDeltaContent,
) {
    if let ToolCallDeltaContent::Delta(delta) = content {
        if let Some((_, _, args)) = pending_tool_calls.last_mut() {
            if let Some(obj) = args.as_object_mut() {
                if let Ok(delta_val) = serde_json::from_str::<serde_json::Value>(&delta) {
                    if let Some(delta_obj) = delta_val.as_object() {
                        for (k, v) in delta_obj { obj.insert(k.clone(), v.clone()); }
                    }
                }
            }
        }
    }
}

async fn handle_reasoning_chunk(
    reasoning: String,
    thinking_content: &mut String,
    is_thinking: &mut bool,
    event_tx: &mpsc::Sender<AgentEvent>,
    turn: usize,
) -> Result<(), AgentLoopError> {
    if !*is_thinking {
        *is_thinking = true;
        *thinking_content = reasoning.clone();
        event_tx.send(AgentEvent::ThinkingStart { turn }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    } else {
        if !thinking_content.is_empty() {
            thinking_content.push(' ');
        }
        thinking_content.push_str(&reasoning);
    }
    event_tx.send(AgentEvent::ThinkingUpdate { text: reasoning, turn }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))
}

async fn handle_reasoning_delta(
    reasoning: String,
    thinking_content: &mut String,
    is_thinking: &mut bool,
    event_tx: &mpsc::Sender<AgentEvent>,
    turn: usize,
) -> Result<(), AgentLoopError> {
    if !*is_thinking {
        *is_thinking = true;
        *thinking_content = reasoning.clone();
        event_tx.send(AgentEvent::ThinkingStart { turn }).await
            .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    } else {
        if !thinking_content.is_empty() {
            thinking_content.push(' ');
        }
        thinking_content.push_str(&reasoning);
    }
    event_tx.send(AgentEvent::ThinkingUpdate { text: reasoning, turn }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))
}

async fn finalize_turn_no_tools(
    event_tx: mpsc::Sender<AgentEvent>,
    assistant_message: AgentMessage,
) -> Result<(), AgentLoopError> {
    event_tx.send(AgentEvent::TurnEnd { message: assistant_message.clone(), tool_results: vec![] }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    event_tx.send(AgentEvent::AgentEnd { messages: vec![assistant_message] }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
    Ok(())
}

async fn finalize_turn_with_tools(
    event_tx: mpsc::Sender<AgentEvent>,
    assistant_message: AgentMessage,
    tool_results: Vec<ToolResult>,
) -> Result<(), AgentLoopError> {
    event_tx.send(AgentEvent::TurnEnd { message: assistant_message.clone(), tool_results }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

    event_tx.send(AgentEvent::AgentEnd { messages: vec![assistant_message] }).await
        .map_err(|e| AgentLoopError::SendError(e.to_string()))?;

    Ok(())
}
