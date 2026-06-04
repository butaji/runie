//! Turn execution logic.

use crate::events::*;
use crate::tools::AgentTool;
use crate::Hook;
use futures::StreamExt;
use runie_ai::Provider;
use runie_core::{Event as LlmEvent, ToolSchema};
use runie_tools::ToolRegistry;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use super::permission_state::PermissionState;
use super::streaming::{start_chat_with_retry, send_event, process_stream_event, PartialToolCall, finalize_tool_calls};
use super::tools::process_tool_calls;
use tokio::sync::mpsc;

/// Main agent loop entry point.
pub async fn run_agent_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    initial_messages: Vec<AgentMessage>,
    config: super::AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    msg_tx: mpsc::Sender<M>,
    permission_state: Arc<PermissionState>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<Vec<AgentMessage>, super::AgentLoopError> {
    tracing::info!("[ACTOR:AgentLoop] Loop started, provider={}, model={}, max_turns={}",
        config.model.split('/').next().unwrap_or("unknown"),
        config.model.split('/').last().unwrap_or(&config.model),
        config.max_turns);

    let mut messages = initial_messages;
    let allowed_tools: HashSet<String> = HashSet::new();
    let context_window = 128_000;

    let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| ToolSchema {
        name: t.name.clone(),
        description: t.description.clone(),
        parameters: t.parameters.clone(),
    }).collect();

    let mut turn_count = 0;
    let max_turns = config.max_turns;

    loop {
        turn_count += 1;

        if turn_count > max_turns {
            return handle_max_turns_exceeded(&msg_tx, max_turns).await;
        }

        handle_context_compaction(&mut messages, &provider, &msg_tx).await;

        let llm_messages = super::context::build_llm_messages(&config.system_prompt, &messages);
        tracing::info!("[ACTOR:AgentLoop] Sending {} messages to LLM", messages.len());
        super::context::log_message_content(&messages);

        let should_continue = execute_turn(
            &mut messages,
            &provider,
            &llm_messages,
            &tool_schemas,
            turn_count,
            &msg_tx,
            &tools,
            &allowed_tools,
            permission_state.clone(),
            registry.clone(),
            hooks.clone(),
            &mut allowed_tools.clone(),
            context_window,
        ).await;

        if !should_continue {
            break;
        }
    }

    send_agent_end(&msg_tx, &messages, turn_count).await;
    Ok(messages)
}

async fn handle_max_turns_exceeded<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    max_turns: usize,
) -> Result<Vec<AgentMessage>, super::AgentLoopError> {
    let (error_type, recoverable, context) = super::classify_error(&super::AgentLoopError::MaxTurnsExceeded);
    send_event(msg_tx, AgentEvent::Error {
        message: format!("Max turns ({}) exceeded", max_turns),
        error_type,
        recoverable,
        context,
    }).await;
    Err(super::AgentLoopError::MaxTurnsExceeded)
}

async fn handle_context_compaction<M: TryFrom<AgentEvent> + Send + 'static>(
    messages: &mut Vec<AgentMessage>,
    provider: &Arc<dyn Provider>,
    msg_tx: &mpsc::Sender<M>,
) {
    if messages.len() > super::context::COMPACT_THRESHOLD {
        tracing::info!("[COMPACT] Context length {}, compacting...", messages.len());
        match super::context::compact_context(messages, provider.clone()).await {
            Ok((compacted_count, summary_preview)) => {
                send_event(msg_tx, AgentEvent::ContextCompacted {
                    original_count: messages.len(),
                    compacted_count,
                    summary_preview: summary_preview.clone(),
                }).await;
                tracing::info!("[COMPACT] Compaction complete: {} -> {}", messages.len(), compacted_count);
            }
            Err(e) => {
                tracing::warn!("[COMPACT] Failed to compact context: {}", e);
                send_event(msg_tx, AgentEvent::Error {
                    message: format!("Context compaction failed: {}", e),
                    error_type: "context".to_string(),
                    recoverable: true,
                    context: e.to_string(),
                }).await;
            }
        }
    }
}

async fn execute_turn<M: TryFrom<AgentEvent> + Send + 'static>(
    messages: &mut Vec<AgentMessage>,
    provider: &Arc<dyn Provider>,
    llm_messages: &Vec<runie_core::Message>,
    tool_schemas: &[ToolSchema],
    turn_count: usize,
    msg_tx: &mpsc::Sender<M>,
    tools: &[AgentTool],
    allowed_tools: &HashSet<String>,
    permission_state: Arc<PermissionState>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
    allowed_tools_mut: &mut HashSet<String>,
    context_window: usize,
) -> bool {
    let stream = match start_chat_with_retry(provider.clone(), llm_messages.clone(), tool_schemas.to_vec()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("[ACTOR:AgentLoop] Stream error: {}", e);
            let (error_type, recoverable, context) = super::classify_error(&super::AgentLoopError::ProviderError(e.to_string()));
            send_event(msg_tx, AgentEvent::Error {
                message: format!("Failed to start chat: {}", e),
                error_type,
                recoverable,
                context,
            }).await;
            return false;
        }
    };

    let (assistant_message, pending_tool_calls) =
        process_stream_loop(stream, turn_count, msg_tx).await;

    messages.push(assistant_message.clone());

    if pending_tool_calls.is_empty() {
        end_turn(msg_tx, turn_count, messages).await;
        return false;
    }

    let tool_results = process_tool_calls(
        messages,
        pending_tool_calls,
        turn_count,
        tools,
        allowed_tools,
        msg_tx,
        permission_state,
        registry,
        hooks,
        allowed_tools_mut,
        context_window,
    ).await;

    end_turn_with_tools(msg_tx, turn_count, messages, tool_results.len()).await;
    true
}

async fn send_agent_end<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    messages: &[AgentMessage],
    turn_count: usize,
) {
    send_event(msg_tx, AgentEvent::AgentEnd {
        messages: messages.to_vec(),
        total_turns: turn_count,
        final_token_usage: TokenUsage::default(),
    }).await;
    tracing::info!("[ACTOR:AgentLoop] Loop ended, total_turns={}", turn_count);
}

// =============================================================================
// Stream processing
// =============================================================================

async fn process_stream_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    mut stream: Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>,
    turn_count: usize,
    msg_tx: &mpsc::Sender<M>,
) -> (AgentMessage, HashMap<(String, String), PartialToolCall>) {
    let mut assistant_message = create_initial_message();
    send_message_start(msg_tx, &assistant_message, turn_count).await;

    let mut pending_tool_calls = HashMap::new();
    let mut text_content = String::new();
    let mut text_buffer = String::new();
    let mut thinking_buffer = String::new();
    let mut last_emit = Instant::now();

    stream_loop(
        &mut stream,
        turn_count,
        msg_tx,
        &mut assistant_message,
        &mut pending_tool_calls,
        &mut text_content,
        &mut text_buffer,
        &mut thinking_buffer,
        &mut last_emit,
    ).await;

    finalize_message(
        msg_tx,
        turn_count,
        assistant_message,
        pending_tool_calls,
        text_content,
    ).await
}

fn create_initial_message() -> AgentMessage {
    AgentMessage {
        role: "assistant".to_string(),
        content: vec![ContentPart::Text { text: String::new() }],
        timestamp: chrono::Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

async fn send_message_start<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    message: &AgentMessage,
    turn: usize,
) {
    send_event(msg_tx, AgentEvent::MessageStart {
        message: message.clone(),
        turn,
    }).await;
}

async fn stream_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    stream: &mut Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>,
    turn_count: usize,
    msg_tx: &mpsc::Sender<M>,
    assistant_message: &mut AgentMessage,
    pending_tool_calls: &mut HashMap<(String, String), PartialToolCall>,
    text_content: &mut String,
    text_buffer: &mut String,
    thinking_buffer: &mut String,
    last_emit: &mut Instant,
) {
    while let Some(event) = stream.next().await {
        match event {
            LlmEvent::MessageEnd => {
                flush_text_buffer(
                    msg_tx,
                    assistant_message,
                    turn_count,
                    text_content,
                    text_buffer,
                ).await;
                if !thinking_buffer.is_empty() {
                    send_event(msg_tx, AgentEvent::ThinkingEnd {
                        duration_ms: 0,
                        turn: turn_count,
                    }).await;
                }
                break;
            }
            LlmEvent::Error { message } => {
                tracing::error!("[ACTOR:AgentLoop] Error: {}", message);
                assistant_message.error_message = Some(message.clone());
                send_event(msg_tx, AgentEvent::Error {
                    message: format!("Stream error: {}", message),
                    error_type: "stream".to_string(),
                    recoverable: true,
                    context: message,
                }).await;
                break;
            }
            _ => {
                process_stream_event(
                    event,
                    assistant_message,
                    pending_tool_calls,
                    text_content,
                    text_buffer,
                    thinking_buffer,
                    turn_count,
                    msg_tx,
                    last_emit,
                ).await;
            }
        }
    }
}

async fn flush_text_buffer<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    assistant_message: &mut AgentMessage,
    turn_count: usize,
    text_content: &mut String,
    text_buffer: &mut String,
) {
    if !text_buffer.is_empty() {
        let _delta = std::mem::take(text_buffer);
        assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
        send_event(msg_tx, AgentEvent::MessageUpdate {
            message: assistant_message.clone(),
            delta: String::new(),
            replace: false,
            turn: turn_count,
        }).await;
    }
}

async fn finalize_message<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    turn_count: usize,
    mut assistant_message: AgentMessage,
    pending_tool_calls: HashMap<(String, String), PartialToolCall>,
    text_content: String,
) -> (AgentMessage, HashMap<(String, String), PartialToolCall>) {
    assistant_message.content = vec![ContentPart::Text { text: text_content }];
    finalize_tool_calls(&mut assistant_message, &pending_tool_calls);
    send_event(msg_tx, AgentEvent::MessageEnd {
        message: assistant_message.clone(),
        turn: turn_count,
    }).await;

    (assistant_message, pending_tool_calls)
}

// =============================================================================
// Turn completion
// =============================================================================

async fn end_turn<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    turn_count: usize,
    messages: &[AgentMessage],
) {
    send_event(msg_tx, AgentEvent::TurnEnd {
        turn: turn_count,
        message_count: messages.len(),
        tool_results_count: 0,
        token_usage: TokenUsage::default(),
    }).await;
}

async fn end_turn_with_tools<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    turn_count: usize,
    messages: &[AgentMessage],
    tool_results_count: usize,
) {
    send_event(msg_tx, AgentEvent::TurnEnd {
        turn: turn_count,
        message_count: messages.len(),
        tool_results_count,
        token_usage: TokenUsage::default(),
    }).await;
    tracing::info!("[ACTOR:AgentLoop] turn_count={}, messages={}, tool_results={}", turn_count, messages.len(), tool_results_count);
}
