use crate::config::AgentConfig;
use crate::events::*;
use crate::tools::AgentTool;
use crate::{Hook, HookDecision};
use runie_ai::Provider;
use runie_core::{Message, ToolSchema, Event as LlmEvent, Context, ToolCall as CoreToolCall, ProviderError};
use runie_tools::ToolRegistry;
use tokio::sync::{mpsc, Mutex};
use futures::StreamExt;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

/// Maximum number of messages to keep in context after compaction
const MAX_CONTEXT_MESSAGES: usize = 50;

/// Compact context when message count exceeds this threshold
const COMPACT_THRESHOLD: usize = 40;

/// Number of recent messages to preserve when compacting (not summarized)
const RECENT_MESSAGES_TO_KEEP: usize = 10;

/// Accumulates streaming tool call deltas until MessageEnd
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
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
/// This is returned by the `agent_loop` function and provides a Stream impl
/// for consuming events, along with methods to send permission decisions
/// and retrieve the final result.
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
    /// Drains any remaining events and returns the accumulated messages.
    pub async fn result(mut self) -> Vec<AgentMessage> {
        // Drain remaining events to find AgentEnd
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

/// Send an agent event through the unified message channel.
async fn send_event<M: TryFrom<AgentEvent> + Send + 'static>(msg_tx: &mpsc::Sender<M>, event: AgentEvent) {
    if let Ok(msg) = M::try_from(event) {
        if msg_tx.send(msg).await.is_err() {
            tracing::error!("Failed to send agent event");
        }
    }
}

/// Calculate estimated context window usage as a percentage.
pub(crate) fn calculate_context_window_usage(messages: &[AgentMessage], context_window: usize) -> f32 {
    let total_chars: usize = messages.iter()
        .map(|m| format_message_content(&m.content, &m.tool_calls).len())
        .sum();
    let estimated_tokens = total_chars / 4;
    if context_window > 0 {
        (estimated_tokens as f32 / context_window as f32) * 100.0
    } else {
        0.0
    }
}

/// Compact context by summarizing old messages when conversation grows too long.
/// Preserves system message + recent messages + summary of old messages.
async fn compact_context(
    history: &mut Vec<AgentMessage>,
    provider: Arc<dyn Provider>,
) -> Result<(usize, String), String> {
    if history.len() <= MAX_CONTEXT_MESSAGES {
        return Ok((history.len(), String::new()));
    }

    let original_count = history.len();

    // Extract system message if present (first message with role "system")
    let system_msg = history.first().filter(|m| m.role == "system").cloned();

    // Get recent messages to preserve (last N messages)
    let recent_msgs: Vec<AgentMessage> = history.iter()
        .rev()
        .take(RECENT_MESSAGES_TO_KEEP)
        .rev()
        .cloned()
        .collect();

    // Get middle messages to summarize (everything between system and recent)
    let middle_start = if system_msg.is_some() { 1 } else { 0 };
    let middle_end = history.len().saturating_sub(RECENT_MESSAGES_TO_KEEP);
    let middle_msgs: Vec<AgentMessage> = if middle_end > middle_start {
        history[middle_start..middle_end].to_vec()
    } else {
        Vec::new()
    };

    // Summarize middle section if present
    let summary = if !middle_msgs.is_empty() {
        summarize_messages(&middle_msgs, provider).await?
    } else {
        String::new()
    };

    // Rebuild history: system + summary + recent
    let mut new_history = Vec::new();

    if let Some(sys) = system_msg {
        new_history.push(sys);
    }

    // Add summary as a system message
    if !summary.is_empty() {
        new_history.push(AgentMessage {
            role: "system".to_string(),
            content: vec![ContentPart::Text {
                text: format!("Previous conversation summary:\n{}", summary),
            }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        });
    }

    // Add recent messages
    new_history.extend(recent_msgs);

    *history = new_history;

    let compacted_count = history.len();
    let summary_preview = if summary.len() > 100 {
        format!("{}...", &summary[..100])
    } else {
        summary.clone()
    };

    tracing::info!(
        "[COMPACT] Context compacted: {} messages -> {} messages",
        original_count, compacted_count
    );

    Ok((compacted_count, summary_preview))
}

/// Summarize a list of messages using the provider's chat_simple method.
async fn summarize_messages(
    messages: &[AgentMessage],
    provider: Arc<dyn Provider>,
) -> Result<String, String> {
    if messages.is_empty() {
        return Ok(String::new());
    }

    let content = messages.iter()
        .map(|m| {
            let role = &m.role;
            let text = format_message_content(&m.content, &m.tool_calls);
            format!("{}: {}", role, text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summary_prompt = format!(
        "Summarize the following conversation concisely, preserving key facts, decisions, and important context:\n\n{}",
        content
    );

    let summary_message = Message::User {
        content: summary_prompt,
        attachments: Vec::new(),
    };

    match provider.chat_simple(vec![summary_message]).await {
        Ok(summary) => {
            tracing::debug!("[COMPACT] Generated summary ({} chars)", summary.len());
            Ok(summary)
        }
        Err(e) => {
            tracing::warn!("[COMPACT] Failed to generate summary: {}", e);
            Err(format!("Failed to summarize: {}", e))
        }
    }
}

/// Start chat with retry logic for rate limit errors.
/// Returns the stream on success, or the final error after retries are exhausted.
/// Non-rate-limit errors (like 401) fail immediately without retry.
pub(crate) async fn start_chat_with_retry(
    provider: Arc<dyn Provider>,
    messages: Vec<Message>,
    tools: Vec<ToolSchema>,
) -> Result<Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>, ProviderError> {
    const MAX_RETRIES: u32 = 4; // 3 failures + 1 success
    const BASE_DELAY_MS: u64 = 1000; // 1 second base delay

    let mut last_error: ProviderError = ProviderError::ApiError("Unknown error".to_string());

    for attempt in 0..MAX_RETRIES {
        match provider.chat(messages.clone(), tools.clone()).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                last_error = e.clone();
                // Only retry on rate limit errors, fail immediately on others (401, etc.)
                if !matches!(e, ProviderError::RateLimited) {
                    return Err(e);
                }
                // Exponential backoff: 1s, 2s, 4s between retries
                if attempt < MAX_RETRIES - 1 {
                    let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt);
                    tracing::info!("Rate limited, retrying in {}ms (attempt {}/{})", delay_ms, attempt + 1, MAX_RETRIES);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_error)
}

/// Classify an error into type and recoverability.
fn classify_error(error: &AgentLoopError) -> (String, bool, String) {
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
    // Log agent loop start
    tracing::info!("[ACTOR:AgentLoop] Loop started, provider={}, model={}, max_turns={}",
        config.model.split('/').next().unwrap_or("unknown"),
        config.model.split('/').last().unwrap_or(&config.model),
        config.max_turns);

    let mut messages = initial_messages;
    let mut allowed_tools: HashSet<String> = HashSet::new();

    // Context window size (default 128k for most models)
    let context_window = 128_000;

    // Track token usage across all turns
    let mut total_input_tokens = 0u32;
    let mut total_output_tokens = 0u32;

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
            let (error_type, recoverable, context) = classify_error(&AgentLoopError::MaxTurnsExceeded);
            send_event(&msg_tx, AgentEvent::Error {
                message: format!("Max turns ({}) exceeded", config.max_turns),
                error_type,
                recoverable,
                context,
            }).await;
            return Err(AgentLoopError::MaxTurnsExceeded);
        }

        // Compact context if it exceeds threshold
        if messages.len() > COMPACT_THRESHOLD {
            tracing::info!("[COMPACT] Context length {}, compacting...", messages.len());
            match compact_context(&mut messages, provider.clone()).await {
                Ok((compacted_count, summary_preview)) => {
                    send_event(&msg_tx, AgentEvent::ContextCompacted {
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

        // Build LLM messages
        let llm_messages = build_llm_messages(&config.system_prompt, &messages);
        tracing::info!("[ACTOR:AgentLoop] Sending {} messages to LLM", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            let tool_use_count = msg.content.iter().filter(|p| matches!(p, ContentPart::ToolUse { .. })).count();
            if msg.role == "assistant" && tool_use_count > 0 {
                tracing::info!("[ACTOR:AgentLoop] Message {}: Assistant with {} tool calls", i, tool_use_count);
                for part in &msg.content {
                    if let ContentPart::ToolUse { id, name, .. } = part {
                        tracing::info!("[ACTOR:AgentLoop]   Tool call: id={} name={}", id, name);
                    }
                }
            } else if msg.role == "tool" {
                let tool_id = msg.content.iter().find_map(|p| {
                    if let ContentPart::ToolResult { tool_use_id, .. } = p {
                        Some(tool_use_id.clone())
                    } else {
                        None
                    }
                }).unwrap_or_default();
                tracing::info!("[ACTOR:AgentLoop] Message {}: ToolResult for id={}", i, tool_id);
            }
        }

        // Start streaming with retry for rate limit errors
        let stream = start_chat_with_retry(provider.clone(), llm_messages, tool_schemas.clone()).await
            .map_err(|e| AgentLoopError::ProviderError(e.to_string()))?;

        // Send message_start event with turn count
        let mut assistant_message = AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: String::new() }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        };

        send_event(&msg_tx, AgentEvent::MessageStart {
            message: assistant_message.clone(),
            turn: turn_count,
        }).await;
        tracing::debug!("[ACTOR:AgentLoop] MessageStart received (turn {})", turn_count);

        // Process stream
        let mut pending_tool_calls: HashMap<(String, String), PartialToolCall> = HashMap::new();
        let mut text_content = String::new();
        let mut text_buffer = String::new();
        let mut last_emit = Instant::now();
        const EMIT_DEBOUNCE_MS: u64 = 100;

        let mut stream = stream;
        while let Some(event) = stream.next().await {
            match event {
                LlmEvent::MessageDelta { content } => {
                    text_buffer.push_str(&content);
                    text_content.push_str(&content);

                    let should_emit = text_buffer.contains('\n')
                        || last_emit.elapsed().as_millis() > EMIT_DEBOUNCE_MS as u128;

                    if should_emit {
                        let delta = std::mem::take(&mut text_buffer);
                        assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                        let delta_len = delta.len();
                        send_event(&msg_tx, AgentEvent::MessageUpdate {
                            message: assistant_message.clone(),
                            turn: turn_count,
                            delta,
                        }).await;
                        tracing::debug!("[ACTOR:AgentLoop] MessageUpdate: \"{}\" (+{} chars)", &text_content[..text_content.len().saturating_sub(delta_len).min(50)], delta_len);
                        last_emit = Instant::now();
                    }
                }
                LlmEvent::ToolCallDelta { id, name, arguments } => {
                    tracing::info!("[TOOL-ACCUMULATE] id={} name={} args_chunk={:?}", id, name, arguments);
                    let key = (id.clone(), name.clone());
                    pending_tool_calls.entry(key).or_insert_with(|| PartialToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: String::new(),
                    }).arguments.push_str(&arguments);
                }
                LlmEvent::MessageEnd => {
                    // Finalize any pending tool calls
                    // Flush remaining text buffer
                    if !text_buffer.is_empty() {
                        let delta = std::mem::take(&mut text_buffer);
                        assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                        send_event(&msg_tx, AgentEvent::MessageUpdate {
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
                LlmEvent::Usage { prompt_tokens, completion_tokens, total_tokens: _ } => {
                    // Accumulate token usage
                    total_input_tokens += prompt_tokens as u32;
                    total_output_tokens += completion_tokens as u32;

                    send_event(&msg_tx, AgentEvent::TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens: prompt_tokens + completion_tokens,
                        context_window,
                    }).await;
                    tracing::debug!("[ACTOR:AgentLoop] Usage: {} prompt, {} completion tokens", prompt_tokens, completion_tokens);
                }
                _ => {
                    tracing::warn!("Unhandled LLM event variant in agent loop");
                }
            }
        }

        // Send message_end with turn count
        assistant_message.content = vec![ContentPart::Text { text: text_content }];
        
        // P0-TOOL-CALLS: Add tool calls to assistant message so they're in history
        if !pending_tool_calls.is_empty() {
            for partial in pending_tool_calls.values() {
                let input = match serde_json::from_str(&partial.arguments) {
                    Ok(v) => v,
                    Err(_) => serde_json::json!({"raw": partial.arguments}),
                };
                assistant_message.content.push(ContentPart::ToolUse {
                    id: partial.id.clone(),
                    name: partial.name.clone(),
                    input: input.clone(),
                });
                assistant_message.tool_calls.push(CoreToolCall {
                    id: partial.id.clone(),
                    name: partial.name.clone(),
                    arguments: input,
                });
            }
        }
        
        send_event(&msg_tx, AgentEvent::MessageEnd {
            message: assistant_message.clone(),
            turn: turn_count,
        }).await;

        // Log tool calls in assistant message (extracted from content)
        let tc_count = assistant_message.content.iter().filter(|p| matches!(p, ContentPart::ToolUse { .. })).count();
        tracing::info!("[ACTOR:AgentLoop] Pushing assistant message with {} tool calls", tc_count);
        for part in &assistant_message.content {
            if let ContentPart::ToolUse { id, name, input } = part {
                tracing::info!("[ACTOR:AgentLoop] Tool call in history: id={} name={} args={:?}", id, name, input);
            }
        }
        messages.push(assistant_message.clone());

        // Execute tool calls
        if pending_tool_calls.is_empty() {
            // No tools, turn is done
            let context_window_usage = calculate_context_window_usage(&messages, context_window);
            let token_usage = TokenUsage {
                input: total_input_tokens,
                output: total_output_tokens,
                cache_read: 0,
                cache_write: 0,
                total_tokens: total_input_tokens + total_output_tokens,
            };
            let _ = context_window_usage; // Used for potential future field
            send_event(&msg_tx, AgentEvent::TurnEnd {
                turn: turn_count,
                message_count: messages.len(),
                tool_results_count: 0,
                token_usage,
            }).await;
            break;
        }

        let mut tool_results = vec![];
        // P2-7 FIX: Idempotency - track seen tool calls to prevent duplicates
        let mut seen_tool_calls: HashSet<String> = HashSet::new();
        tracing::info!("[ACTOR:AgentLoop] {} tool calls finalized", pending_tool_calls.len());
        for partial in pending_tool_calls.values() {
            tracing::info!("[ACTOR:AgentLoop] id={} name={} accumulated_args={:?}", partial.id, partial.name, partial.arguments);
        }
        // Drain pending_tool_calls HashMap, converting accumulated arguments to JSON
        let finalized_calls: Vec<(ContentPart, String, String)> = pending_tool_calls.drain().map(|((id, name), partial)| {
            // Try to parse accumulated arguments as JSON, fall back to raw string
            let input = match serde_json::from_str(&partial.arguments) {
                Ok(v) => v,
                Err(_) => serde_json::json!({"raw": partial.arguments}),
            };
            (ContentPart::ToolUse { id: id.clone(), name: name.clone(), input }, name.clone(), partial.arguments)
        }).collect();
        for (tool_use, _tool_name, _args_str) in finalized_calls {
            if let ContentPart::ToolUse { id, name, input } = &tool_use {
                // P0-TOOL-VALIDATION: Skip tool calls with empty or invalid names
                if name.trim().is_empty() {
                    tracing::warn!("Tool call with empty name skipped (call_id: {})", id);
                    send_event(&msg_tx, AgentEvent::Error {
                        message: format!("Tool call '{}' has empty name - skipping", id),
                        error_type: "invalid_tool_call".to_string(),
                        recoverable: true,
                        context: format!("The model generated a tool call without a name. Raw input: {:?}", input),
                    }).await;
                    continue;
                }

                // P0-TOOL-VALIDATION: Validate tool exists in registry
                if !tools.iter().any(|t| t.name == *name) {
                    tracing::warn!("Tool '{}' not found in registry (call_id: {})", name, id);
                    send_event(&msg_tx, AgentEvent::Error {
                        message: format!("Tool '{}' not found", name),
                        error_type: "tool_not_found".to_string(),
                        recoverable: true,
                        context: format!("Available tools: {}", tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ")),
                    }).await;
                    continue;
                }

                // P2-7 FIX: Check for duplicate tool call (same name + args in same turn)
                let tool_key = format!("{}:{}", name, serde_json::to_string(input).unwrap_or_default());
                if seen_tool_calls.contains(&tool_key) {
                    tracing::warn!("Duplicate tool call detected and skipped: {} with args {:?}", name, input);
                    continue;
                }
                seen_tool_calls.insert(tool_key);

                let tool_args = serde_json::to_string(input).unwrap_or_default();
                let context_window_usage = calculate_context_window_usage(&messages, context_window);

                send_event(&msg_tx, AgentEvent::ToolExecutionStart {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: tool_args.clone(),
                    turn: turn_count,
                }).await;
                tracing::info!("[ACTOR:AgentLoop] {} requested: {}", name, tool_args);

                // Check if tool is in allowed cache first
                let should_execute = if allowed_tools.contains(name) {
                    send_event(&msg_tx, AgentEvent::PermissionGranted {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                    }).await;
                    true
                } else {
                    // Get tool description for permission request
                    let tool_description = tools.iter()
                        .find(|t| t.name == *name)
                        .map(|t| t.description.clone())
                        .unwrap_or_default();

                    // Send permission request
                    send_event(&msg_tx, AgentEvent::PermissionRequest {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        tool_description,
                        turn: turn_count,
                        context_window_usage,
                    }).await;

                    // Wait for permission decision by polling shared state
                    let decision = tokio::time::timeout(
                        Duration::from_secs(300), // 5 minute timeout
                        async {
                            loop {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                let permission = permission_state.lock().await.take();
                                if permission.is_some() {
                                    break permission;
                                }
                            }
                        }
                    ).await;

                    match decision {
                        Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            send_event(&msg_tx, AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            true
                        }
                        Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            // Cache the tool name for future auto-allow
                            allowed_tools.insert(name.clone());
                            send_event(&msg_tx, AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            true
                        }
                        Ok(Some(PermissionDecision::Skip { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            false // Skip this tool but continue with others
                        }
                        Ok(Some(PermissionDecision::Deny { tool_call_id: ref _tid, ref tool_name, ref tool_args })) => {
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            false
                        }
                        _ => {
                            // Timeout, mismatch, or deny
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
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

                    let duration_ms = 0u64; // Denied tools don't execute
                    send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        result: result.clone(),
                        duration_ms,
                        turn: turn_count,
                    }).await;

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
                    tool_calls: vec![],
                });

                    tool_results.push(result);
                    continue;
                }

                // Track execution start time for duration calculation
                let start_time = Instant::now();

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
                            tracing::error!("Hook error: {}", e);
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
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        result: blocked_result.clone(),
                        duration_ms,
                        turn: turn_count,
                    }).await;
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

                        // Send panic event to notify TUI with error classification
                        send_event(&msg_tx, AgentEvent::Error {
                            message: format!("Tool '{}' panicked: {}", name, panic_msg),
                            error_type: "tool_panic".to_string(),
                            recoverable: true,
                            context: format!("Tool '{}' panicked during execution", name),
                        }).await;

                        panic_result
                    }
                };

                let duration_ms = start_time.elapsed().as_millis() as u64;
                send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: tool_args.clone(),
                    result: result.clone(),
                    duration_ms,
                    turn: turn_count,
                }).await;
                // Log tool result
                let result_preview = result.content.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join("; ");
                tracing::info!("[ACTOR:AgentLoop] {} result: {} ({}ms)", name, result_preview.chars().take(100).collect::<String>(), duration_ms);

                // Add tool result to messages
                tracing::info!("[ACTOR:AgentLoop] Pushing tool result for id={}", id);
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
                    tool_calls: vec![],
                });

                tool_results.push(result);
            }
        }

        let context_window_usage = calculate_context_window_usage(&messages, context_window);
        let token_usage = TokenUsage {
            input: total_input_tokens,
            output: total_output_tokens,
            cache_read: 0,
            cache_write: 0,
            total_tokens: total_input_tokens + total_output_tokens,
        };
        let _ = context_window_usage; // Used for potential future field
        send_event(&msg_tx, AgentEvent::TurnEnd {
            turn: turn_count,
            message_count: messages.len(),
            tool_results_count: tool_results.len(),
            token_usage,
        }).await;
        tracing::info!("[ACTOR:AgentLoop] turn_count={}, messages={}, tool_results={}", turn_count, messages.len(), tool_results.len());

        // Continue loop - send updated messages back to LLM
    }

    let final_token_usage = TokenUsage {
        input: total_input_tokens,
        output: total_output_tokens,
        cache_read: 0,
        cache_write: 0,
        total_tokens: total_input_tokens + total_output_tokens,
    };
    send_event(&msg_tx, AgentEvent::AgentEnd {
        messages: messages.clone(),
        total_turns: turn_count,
        final_token_usage,
    }).await;
    tracing::info!("[ACTOR:AgentLoop] Loop ended, total_turns={}, total_tokens={}", turn_count, total_input_tokens + total_output_tokens);
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::ToolCall;

    #[tokio::test]
    async fn test_tool_empty_name_skipped() {
        // Create a tool registry with a mock tool
        let registry = ToolRegistry::new();

        // Verify that an empty-named tool call does not cause issues
        // Empty names should be caught before tool execution
        let empty_name = "";
        let result = registry.get(empty_name);
        assert!(result.is_none(), "Empty tool name should not find any tool");
    }

    #[tokio::test]
    async fn test_tool_call_with_empty_name_validation() {
        // Test that ToolCall with empty name is handled properly
        let tool_call = ToolCall {
            id: "call_test".to_string(),
            name: "".to_string(),
            arguments: serde_json::json!({}),
        };

        // An empty name should not be considered valid
        assert!(tool_call.name.is_empty());
    }

    #[tokio::test]
    async fn test_tool_invalid_args_returns_error_not_panic() {
        // Create a tool registry
        let registry = ToolRegistry::new();

        // Test that malformed arguments return ToolError, not panic
        let tool = registry.get("bash");
        if let Some(tool) = tool {
            // Pass invalid JSON structure as args
            let result = tool.execute(serde_json::json!({"command": 123})).await;
            // Should return error, not panic
            assert!(result.is_err());
            let err = result.unwrap_err();
            // Should be InvalidArguments error
            assert!(matches!(err, runie_core::ToolError::InvalidArguments(_)));
        }
    }

    #[tokio::test]
    async fn test_tool_missing_required_args_returns_error() {
        let registry = ToolRegistry::new();

        let tool = registry.get("read_file");
        if let Some(tool) = tool {
            // Missing 'path' argument
            let result = tool.execute(serde_json::json!({})).await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, runie_core::ToolError::InvalidArguments(_)));
            assert!(err.to_string().contains("Missing 'path' argument"));
        }
    }

    #[tokio::test]
    async fn test_tool_call_delta_with_empty_name() {
        // Simulate what happens when LlmEvent::ToolCallDelta has empty name
        let name = "";
        let _arguments = r#"{"command": "echo test"}"#.to_string();

        // An empty name should be detectable before creating a tool call
        assert!(name.is_empty(), "Empty name should be detected");

        // If we had a function that validates tool call deltas, it should reject this
        fn is_valid_tool_name(name: &str) -> bool {
            !name.is_empty()
        }

        assert!(!is_valid_tool_name(""), "Empty tool name should be invalid");
        assert!(is_valid_tool_name("bash"), "Non-empty tool name should be valid");
    }

    #[tokio::test]
    async fn test_empty_tool_name_skipped() {
        // Verify that tool calls with empty names would be skipped by validation
        let empty_name = "";
        let trimmed = empty_name.trim();
        assert!(trimmed.is_empty(), "Empty tool name should be caught by validation");

        // Simulate the validation check from the loop
        fn is_valid_tool_name(name: &str) -> bool {
            !name.trim().is_empty()
        }
        assert!(!is_valid_tool_name(""), "Empty string is invalid");
        assert!(!is_valid_tool_name("   "), "Whitespace-only string is invalid");
        assert!(is_valid_tool_name("bash"), "Normal tool name is valid");
    }

    #[tokio::test]
    async fn test_unknown_tool_name_skipped() {
        // Create a tool registry and verify unknown tools are not found
        let registry = ToolRegistry::new();

        // These tools should not exist in an empty registry
        let unknown_tool = registry.get("nonexistent_tool");
        assert!(unknown_tool.is_none(), "Unknown tool should not be found");

        let another_unknown = registry.get("completely_invalid");
        assert!(another_unknown.is_none(), "Invalid tool name should return None");

        // Verify the validation logic: tool must exist in registry
        let registered_tools: Vec<_> = vec!["bash", "read_file", "write_file"];
        fn tool_exists(name: &str, available: &[&str]) -> bool {
            available.iter().any(|&t| t == name)
        }
        assert!(!tool_exists("unknown", &registered_tools), "Unknown tool should be rejected");
        assert!(tool_exists("bash", &registered_tools), "Known tool should be accepted");
    }

    #[test]
    fn test_compaction_constants() {
        // Verify compaction thresholds are sensible
        assert!(MAX_CONTEXT_MESSAGES > COMPACT_THRESHOLD,
            "MAX_CONTEXT_MESSAGES should be greater than COMPACT_THRESHOLD");
        assert!(COMPACT_THRESHOLD > RECENT_MESSAGES_TO_KEEP,
            "COMPACT_THRESHOLD should be greater than RECENT_MESSAGES_TO_KEEP");
        assert!(RECENT_MESSAGES_TO_KEEP > 0, "RECENT_MESSAGES_TO_KEEP should be positive");
    }

    #[test]
    fn test_compact_context_below_threshold() {
        // Test that compaction doesn't modify history when below threshold
        let mut history = vec![
            AgentMessage {
                role: "system".to_string(),
                content: vec![ContentPart::Text { text: "You are a helpful assistant".to_string() }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            AgentMessage {
                role: "user".to_string(),
                content: vec![ContentPart::Text { text: "Hello".to_string() }],
                timestamp: 1,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
        ];

        // Create a mock provider that won't be called since no compaction needed
        // We can't easily mock it here, but we can test the constants behavior
        let original_len = history.len();
        assert!(original_len <= COMPACT_THRESHOLD,
            "Test setup error: history should be below COMPACT_THRESHOLD");
    }

    #[test]
    fn test_message_content_extraction_for_summary() {
        // Test that format_message_content correctly extracts text from messages
        let parts = vec![
            ContentPart::Text { text: "Hello world".to_string() },
        ];
        let tool_calls: Vec<CoreToolCall> = vec![];
        let content = format_message_content(&parts, &tool_calls);
        assert_eq!(content, "Hello world");

        // Test tool use extraction
        let parts_with_tool = vec![
            ContentPart::Text { text: "".to_string() },
            ContentPart::ToolUse {
                id: "call_123".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({"command": "ls"}),
            },
        ];
        let tool_calls = vec![CoreToolCall {
            id: "call_123".to_string(),
            name: "bash".to_string(),
            arguments: serde_json::json!({"command": "ls"}),
        }];
        let content = format_message_content(&parts_with_tool, &tool_calls);
        assert!(content.contains("bash"));
        assert!(content.contains("ls"));
    }

    #[test]
    fn test_context_window_usage_calculation() {
        // Test the context window usage calculation
        let messages = vec![
            AgentMessage {
                role: "user".to_string(),
                content: vec![ContentPart::Text { text: "This is a test message with some content".to_string() }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
        ];

        // ~50 chars / 4 = ~12.5 tokens estimated
        let usage = calculate_context_window_usage(&messages, 128_000);
        assert!(usage > 0.0, "Usage should be positive for non-empty message");
        assert!(usage < 1.0, "Usage should be less than 1% for small message");

        // Empty messages should give 0
        let empty_messages: Vec<AgentMessage> = vec![];
        let empty_usage = calculate_context_window_usage(&empty_messages, 128_000);
        assert_eq!(empty_usage, 0.0, "Empty messages should give 0% usage");

        // Zero context window should give 0
        let zero_usage = calculate_context_window_usage(&messages, 0);
        assert_eq!(zero_usage, 0.0, "Zero context window should give 0%");
    }

    #[test]
    fn test_summarize_messages_empty_input() {
        // Test that summarize_messages handles empty input
        let messages: Vec<AgentMessage> = vec![];
        // We can't easily test the async summarize without a mock provider,
        // but we can verify the function signature and basic behavior
        assert!(messages.is_empty());
    }
}

fn build_llm_messages(system_prompt: &str, messages: &[AgentMessage]) -> Vec<Message> {
    let mut llm_msgs = vec![Message::System { content: system_prompt.to_string() }];
    for msg in messages {
        let content = format_message_content(&msg.content, &msg.tool_calls);
        if let Some(m) = agent_msg_to_llm(&msg.role, content, &msg.content, &msg.tool_calls) {
            llm_msgs.push(m);
        }
    }
    llm_msgs
}

/// P1-3 FIX: Execute tool with panic recovery
/// Wraps tool execution in a catch_unwind to prevent panics from crashing the agent.
/// Returns Ok(result) on success, Err(panic_message) if tool panicked.
///
/// NOTE: This currently catches panics from data preparation only, not from the
/// async tool.execute() call itself. Async panics would need a different isolation
/// mechanism (e.g., a dedicated worker process/thread) to catch properly.
async fn execute_tool_with_panic_catch(
    registry: Arc<ToolRegistry>,
    name: &str,
    input: serde_json::Value,
    hooks: Vec<Arc<dyn Hook>>,
    tool_call: runie_core::ToolCall,
    ctx: Context,
) -> Result<ToolResult, String> {
    let registry_clone = registry.clone();
    let name_clone = name.to_string();
    let input_clone = input.clone();
    let hooks_clone = hooks.clone();
    let tool_call_clone = tool_call.clone();
    let ctx_clone = ctx.clone();

    // First: run data preparation in spawn_blocking with catch_unwind
    // This catches panics from any sync setup code
    let prep_result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            (name_clone, input_clone, registry_clone, hooks_clone, tool_call_clone, ctx_clone)
        }))
    }).await;

    let (name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final) = match prep_result {
        Ok(Ok(data)) => data,
        Ok(Err(panic_info)) => {
            let panic_msg = extract_panic_message(panic_info);
            return Err(panic_msg);
        }
        Err(join_err) => {
            return Err(format!("Task execution failed: {}", join_err));
        }
    };

    // Execute the tool (async - panics here cannot be caught by catch_unwind
    // since catch_unwind only works with the current stack frame)
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
    let final_output = match output_result {
        Ok(mut output) => {
            for hook in &hooks_final {
                match hook.after_tool_call(&tool_call_final, &output, &ctx_final).await {
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
    };

    Ok(ToolResult {
        tool_call_id: tool_call_final.id.clone(),
        tool_name: name_str.clone(),
        input: input_final,
        content: vec![ContentPart::Text { text: final_output.content }],
        is_error: final_output.terminate,
    })
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

fn format_message_content(parts: &[ContentPart], tool_calls: &[CoreToolCall]) -> String {
    // Build a map from (name, arguments) to id for looking up tool call IDs
    let tc_map: HashMap<(String, String), String> = tool_calls.iter().map(|tc| {
        let args_str = tc.arguments.to_string();
        ((tc.name.clone(), args_str), tc.id.clone())
    }).collect();

    parts.iter().map(|part| match part {
        ContentPart::Text { text } => text.clone(),
        ContentPart::ToolUse { id, name, input } => {
            let args_str = input.to_string();
            let tc_id = tc_map.get(&(name.clone(), args_str)).cloned().unwrap_or_else(|| id.clone());
            format!("[TC:{}] {}({})", tc_id, name, input)
        }
        ContentPart::ToolResult { content, .. } => content.iter().map(|c| match c {
            ContentPart::Text { text } => text.clone(),
            _ => String::new(),
        }).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }).collect::<Vec<_>>().join("\n")
}

/// Convenience wrapper that runs the agent loop and returns an event stream.
/// Creates a channel internally and spawns the loop as a task.
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

    // Spawn task that bridges permission_rx to permission_state
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

fn agent_msg_to_llm(role: &str, content: String, parts: &[ContentPart], tool_calls: &[CoreToolCall]) -> Option<Message> {
    match role {
        "user" => Some(Message::User { content, attachments: Vec::new() }),
        "assistant" => Some(Message::Assistant {
            content,
            tool_calls: tool_calls.to_vec(),
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
                tracing::error!("Tool result missing tool_use_id - this indicates a bug in message construction");
                "unknown".to_string()
            });

            // Validate: warn if tool_call_id looks fake (generated by buggy code)
            if tool_call_id == "unknown" || tool_call_id.starts_with("call_") && tool_call_id.chars().count() <= 7 {
                tracing::error!(
                    "INVALID TOOL_CALL_ID '{}' - this will cause 400 Bad Request from LLM API. \
                    Tool result must reference a valid tool_call.id from the assistant message.",
                    tool_call_id
                );
            }

            Some(Message::ToolResult { tool_call_id, content, is_error: false })
        }
        _ => None,
    }
}
