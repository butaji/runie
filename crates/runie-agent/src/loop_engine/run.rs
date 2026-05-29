//! Turn execution logic.

use crate::events::*;
use crate::tools::AgentTool;
use crate::Hook;
use futures::StreamExt;
use permissions::request_permission;
use runie_ai::Provider;
use runie_core::{Event as LlmEvent, ToolSchema};
use runie_tools::ToolRegistry;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use streaming::{start_chat_with_retry, send_event, process_stream_event, PartialToolCall, finalize_tool_calls};
use tools::process_tool_calls;
use tokio::sync::Mutex;

/// Main agent loop entry point.
pub async fn run_agent_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    initial_messages: Vec<AgentMessage>,
    config: super::AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    msg_tx: mpsc::Sender<M>,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<Vec<AgentMessage>, super::AgentLoopError> {
    tracing::info!("[ACTOR:AgentLoop] Loop started, provider={}, model={}, max_turns={}",
        config.model.split('/').next().unwrap_or("unknown"),
        config.model.split('/').last().unwrap_or(&config.model),
        config.max_turns);

    let mut messages = initial_messages;
    let mut allowed_tools: HashSet<String> = HashSet::new();
    let context_window = 128_000;
    let mut total_input_tokens = 0u32;
    let mut total_output_tokens = 0u32;

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

        let llm_messages = crate::context::build_llm_messages(&config.system_prompt, &messages);
        tracing::info!("[ACTOR:AgentLoop] Sending {} messages to LLM", messages.len());
        crate::context::log_message_content(&messages);

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
            total_input_tokens,
            total_output_tokens,
        ).await;

        if !should_continue {
            break;
        }
    }

    send_agent_end(&msg_tx, &messages, turn_count, total_input_tokens, total_output_tokens).await;
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
    if messages.len() > crate::context::COMPACT_THRESHOLD {
        tracing::info!("[COMPACT] Context length {}, compacting...", messages.len());
        match crate::context::compact_context(messages, provider.clone()).await {
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
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
    allowed_tools_mut: &mut HashSet<String>,
    context_window: usize,
    total_input_tokens: u32,
    total_output_tokens: u32,
) -> bool {
    let stream = match start_chat_with_retry(provider.clone(), llm_messages.clone(), tool_schemas.to_vec()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("[ACTOR:AgentLoop] Stream error: {}", e);
            return false;
        }
    };

    let (assistant_message, pending_tool_calls) =
        process_stream_loop(stream, turn_count, msg_tx).await;

    messages.push(assistant_message.clone());

    if pending_tool_calls.is_empty() {
        end_turn(msg_tx, turn_count, messages, total_input_tokens, total_output_tokens).await;
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

    end_turn_with_tools(msg_tx, turn_count, messages, tool_results.len(), total_input_tokens, total_output_tokens).await;
    true
}

async fn send_agent_end<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    messages: &[AgentMessage],
    turn_count: usize,
    total_input_tokens: u32,
    total_output_tokens: u32,
) {
    let final_token_usage = TokenUsage {
        input: total_input_tokens,
        output: total_output_tokens,
        cache_read: 0,
        cache_write: 0,
        total_tokens: total_input_tokens + total_output_tokens,
    };
    send_event(msg_tx, AgentEvent::AgentEnd {
        messages: messages.to_vec(),
        total_turns: turn_count,
        final_token_usage,
    }).await;
    tracing::info!("[ACTOR:AgentLoop] Loop ended, total_turns={}, total_tokens={}", turn_count, total_input_tokens + total_output_tokens);
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
    let mut last_emit = Instant::now();

    stream_loop(
        &mut stream,
        turn_count,
        msg_tx,
        &mut assistant_message,
        &mut pending_tool_calls,
        &mut text_content,
        &mut text_buffer,
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
                break;
            }
            LlmEvent::Error { message } => {
                tracing::error!("[ACTOR:AgentLoop] Error: {}", message);
                assistant_message.error_message = Some(message);
                break;
            }
            _ => {
                process_stream_event(
                    event,
                    assistant_message,
                    pending_tool_calls,
                    text_content,
                    text_buffer,
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
        let delta = std::mem::take(text_buffer);
        assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
        send_event(msg_tx, AgentEvent::MessageUpdate {
            message: assistant_message.clone(),
            turn: turn_count,
            delta,
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
    total_input_tokens: u32,
    total_output_tokens: u32,
) {
    let token_usage = TokenUsage {
        input: total_input_tokens,
        output: total_output_tokens,
        cache_read: 0,
        cache_write: 0,
        total_tokens: total_input_tokens + total_output_tokens,
    };
    send_event(msg_tx, AgentEvent::TurnEnd {
        turn: turn_count,
        message_count: messages.len(),
        tool_results_count: 0,
        token_usage,
    }).await;
}

async fn end_turn_with_tools<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    turn_count: usize,
    messages: &[AgentMessage],
    tool_results_count: usize,
    total_input_tokens: u32,
    total_output_tokens: u32,
) {
    let token_usage = TokenUsage {
        input: total_input_tokens,
        output: total_output_tokens,
        cache_read: 0,
        cache_write: 0,
        total_tokens: total_input_tokens + total_output_tokens,
    };
    send_event(msg_tx, AgentEvent::TurnEnd {
        turn: turn_count,
        message_count: messages.len(),
        tool_results_count,
        token_usage,
    }).await;
    tracing::info!("[ACTOR:AgentLoop] turn_count={}, messages={}, tool_results={}", turn_count, messages.len(), tool_results_count);
}
