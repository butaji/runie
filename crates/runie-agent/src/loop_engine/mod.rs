pub mod context;
pub mod permissions;
pub mod streaming;
pub mod tools;

mod tests;

// Re-export for tests
pub(crate) use streaming::start_chat_with_retry;

use crate::config::AgentConfig;
use crate::events::*;
use crate::tools::AgentTool;
use crate::Hook;
use chrono::Utc;
use context::{compact_context, build_llm_messages, COMPACT_THRESHOLD, log_message_content};
use futures::StreamExt;
use permissions::{request_permission, add_denied_result, add_blocked_result, add_tool_result};
use runie_ai::Provider;
use runie_core::{Event as LlmEvent, ToolCall as CoreToolCall, ToolSchema, Context};
use runie_tools::ToolRegistry;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::Instant;
use streaming::{send_event, process_stream_event, PartialToolCall, finalize_tool_calls};
use tools::{execute_tool_with_panic_catch, run_before_hooks, process_tool_calls};
use tokio::sync::{mpsc, Mutex};

/// Calculate estimated context window usage as a percentage.
pub(crate) fn calculate_context_window_usage(messages: &[AgentMessage], context_window: usize) -> f32 {
    let total_chars: usize = messages.iter()
        .map(|m| context::format_message_content(&m.content, &m.tool_calls).len())
        .sum();
    let estimated_tokens = total_chars / 4;
    if context_window > 0 {
        (estimated_tokens as f32 / context_window as f32) * 100.0
    } else {
        0.0
    }
}

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

/// Event stream for consuming agent loop events and sending permission decisions.
pub struct AgentEventStream {
    rx: mpsc::Receiver<AgentEvent>,
    perm_tx: mpsc::Sender<PermissionDecision>,
    result: Option<Vec<AgentMessage>>,
}

impl AgentEventStream {
    /// Send a permission decision back to the agent loop.
    pub async fn send_permission(
        &self,
        decision: PermissionDecision,
    ) -> Result<(), mpsc::error::SendError<PermissionDecision>> {
        self.perm_tx.send(decision).await
    }

    /// Consume the stream and collect the final result (messages from AgentEnd event).
    pub async fn result(mut self) -> Vec<AgentMessage> {
        while let Ok(event) = self.rx.try_recv() {
            if let AgentEvent::AgentEnd { messages, .. } = event {
                return messages;
            }
        }
        self.result.unwrap_or_default()
    }
}

impl futures::Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Classify an error into type and recoverability.
pub(crate) fn classify_error(error: &AgentLoopError) -> (String, bool, String) {
    match error {
        AgentLoopError::ProviderError(msg) => (
            "provider".to_string(),
            true,
            format!("Provider error: {}", msg),
        ),
        AgentLoopError::ToolError(msg) => (
            "tool".to_string(),
            true,
            format!("Tool error: {}", msg),
        ),
        AgentLoopError::SendError(msg) => (
            "send".to_string(),
            true,
            format!("Send error: {}", msg),
        ),
        AgentLoopError::MaxTurnsExceeded => (
            "max_turns".to_string(),
            false,
            "Maximum number of turns exceeded".to_string(),
        ),
    }
}

pub async fn run_agent_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    msg_tx: mpsc::Sender<M>,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<Vec<AgentMessage>, AgentLoopError> {
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

        // Check turn limit
        if turn_count > max_turns {
            return handle_max_turns_exceeded(&msg_tx, max_turns).await;
        }

        // Compact context if needed
        handle_context_compaction(&mut messages, &provider, &msg_tx).await;

        // Build LLM messages
        let llm_messages = build_llm_messages(&config.system_prompt, &messages);
        tracing::info!("[ACTOR:AgentLoop] Sending {} messages to LLM", messages.len());
        log_message_content(&messages);

        // Execute turn
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
) -> Result<Vec<AgentMessage>, AgentLoopError> {
    let (error_type, recoverable, context) = classify_error(&AgentLoopError::MaxTurnsExceeded);
    send_event(msg_tx, AgentEvent::Error {
        message: format!("Max turns ({}) exceeded", max_turns),
        error_type,
        recoverable,
        context,
    }).await;
    Err(AgentLoopError::MaxTurnsExceeded)
}

async fn handle_context_compaction<M: TryFrom<AgentEvent> + Send + 'static>(
    messages: &mut Vec<AgentMessage>,
    provider: &Arc<dyn Provider>,
    msg_tx: &mpsc::Sender<M>,
) {
    if messages.len() > COMPACT_THRESHOLD {
        tracing::info!("[COMPACT] Context length {}, compacting...", messages.len());
        match compact_context(messages, provider.clone()).await {
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
    // Start streaming with retry
    let stream = match start_chat_with_retry(provider.clone(), llm_messages.clone(), tool_schemas.to_vec()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("[ACTOR:AgentLoop] Stream error: {}", e);
            return false;
        }
    };

    // Process stream
    let (assistant_message, pending_tool_calls) =
        process_stream_loop(stream, turn_count, msg_tx).await;

    messages.push(assistant_message.clone());

    // Execute tools or end turn
    if pending_tool_calls.is_empty() {
        end_turn(msg_tx, turn_count, messages, total_input_tokens, total_output_tokens).await;
        return false;
    }

    // Process tool calls
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

/// Process the LLM stream and return the assistant message, pending tool calls, and text content.
async fn process_stream_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    mut stream: Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>,
    turn_count: usize,
    msg_tx: &mpsc::Sender<M>,
) -> (AgentMessage, HashMap<(String, String), PartialToolCall>) {
    let mut assistant_message = AgentMessage {
        role: "assistant".to_string(),
        content: vec![ContentPart::Text { text: String::new() }],
        timestamp: Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    };

    send_event(msg_tx, AgentEvent::MessageStart {
        message: assistant_message.clone(),
        turn: turn_count,
    }).await;

    let mut pending_tool_calls: HashMap<(String, String), PartialToolCall> = HashMap::new();
    let mut text_content = String::new();
    let mut text_buffer = String::new();
    let mut last_emit = Instant::now();

    while let Some(event) = stream.next().await {
        match event {
            LlmEvent::MessageEnd => {
                // Flush remaining text buffer
                if !text_buffer.is_empty() {
                    let delta = std::mem::take(&mut text_buffer);
                    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                    send_event(msg_tx, AgentEvent::MessageUpdate {
                        message: assistant_message.clone(),
                        turn: turn_count,
                        delta,
                    }).await;
                }
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
                    &mut assistant_message,
                    &mut pending_tool_calls,
                    &mut text_content,
                    &mut text_buffer,
                    turn_count,
                    msg_tx,
                    &mut last_emit,
                ).await;
            }
        }
    }

    // Finalize message
    assistant_message.content = vec![ContentPart::Text { text: text_content }];
    finalize_tool_calls(&mut assistant_message, &pending_tool_calls);
    send_event(msg_tx, AgentEvent::MessageEnd {
        message: assistant_message.clone(),
        turn: turn_count,
    }).await;

    (assistant_message, pending_tool_calls)
}

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

/// Convenience wrapper that runs the agent loop and returns an event stream.
pub fn agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> AgentEventStream {
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(128);
    let (perm_tx, perm_rx) = mpsc::channel::<PermissionDecision>(1);
    let permission_state = Arc::new(Mutex::new(None));
    let permission_state_clone = permission_state.clone();

    tokio::spawn(async move {
        let mut perm_rx = perm_rx;
        while let Some(decision) = perm_rx.recv().await {
            let mut state = permission_state_clone.lock().await;
            *state = Some(decision);
        }
    });

    tokio::spawn(async move {
        let _ = run_agent_loop(
            initial_messages,
            config,
            provider,
            tools,
            event_tx,
            permission_state,
            registry,
            hooks,
        ).await;
    });

    AgentEventStream {
        rx: event_rx,
        perm_tx,
        result: None,
    }
}
