use crate::events::*;
use crate::AgentMessage;
use runie_ai::Provider;
use runie_core::{Event as LlmEvent, ToolCall as CoreToolCall, ToolSchema};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use runie_core::ProviderError;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retries for rate limit errors
    pub max_retries: u32,
    /// Base delay in milliseconds for exponential backoff
    pub base_delay_ms: u64,
    /// Maximum delay cap in seconds (0 = no cap)
    pub max_delay_seconds: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 4,
            base_delay_ms: 1000,
            max_delay_seconds: 60,
        }
    }
}

/// Accumulates streaming tool call deltas until MessageEnd
pub(crate) struct PartialToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Calculate delay with server-guided retry-after and exponential backoff
fn calculate_delay(
    retry_after_seconds: Option<u64>,
    attempt: u32,
    config: &RetryConfig,
) -> Duration {
    // Server-provided retry-after takes precedence
    if let Some(retry_after) = retry_after_seconds {
        let delay = if config.max_delay_seconds > 0 {
            retry_after.min(config.max_delay_seconds)
        } else {
            retry_after
        };
        tracing::info!("Using server-provided retry-after: {}s", delay);
        return Duration::from_secs(delay);
    }

    // Fall back to exponential backoff: 1s, 2s, 4s, 8s...
    let backoff_ms = config.base_delay_ms * 2u64.pow(attempt);

    // Apply max delay cap if configured
    let delay_ms = if config.max_delay_seconds > 0 {
        let max_ms = config.max_delay_seconds * 1000;
        backoff_ms.min(max_ms)
    } else {
        backoff_ms
    };

    Duration::from_millis(delay_ms)
}

/// Start chat with retry logic for rate limit errors.
/// Returns the stream on success, or the final error after retries are exhausted.
/// Non-rate-limit errors (like 401) fail immediately without retry.
///
/// Supports server-guided retry via `retry-after` header through RateLimitedRetryAfter variant.
pub(crate) async fn start_chat_with_retry(
    provider: Arc<dyn Provider>,
    messages: Vec<runie_core::Message>,
    tools: Vec<ToolSchema>,
) -> Result<Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>, ProviderError> {
    start_chat_with_retry_with_config(provider, messages, tools, &RetryConfig::default()).await
}

/// Maximum time to wait for a single `provider.chat()` call to return the stream head
/// before treating it as a timeout. Streaming itself is bounded by the consumer.
const CHAT_CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

/// Start chat with custom retry configuration
pub(crate) async fn start_chat_with_retry_with_config(
    provider: Arc<dyn Provider>,
    messages: Vec<runie_core::Message>,
    tools: Vec<ToolSchema>,
    config: &RetryConfig,
) -> Result<Pin<Box<dyn futures::Stream<Item = LlmEvent> + Send + 'static>>, ProviderError> {
    let mut last_error: ProviderError = ProviderError::ApiError("Unknown error".to_string());

    for attempt in 0..config.max_retries {
        match tokio::time::timeout(
            CHAT_CONNECT_TIMEOUT,
            provider.chat(messages.clone(), tools.clone()),
        ).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => {
                last_error = e.clone();

                // Only retry on rate limit errors, fail immediately on others (401, etc.)
                if !e.is_rate_limited() {
                    return Err(e);
                }

                // Get retry-after duration if server-provided
                let retry_after = e.retry_after_seconds();

                if attempt < config.max_retries - 1 {
                    let delay = calculate_delay(retry_after, attempt, config);
                    tracing::info!(
                        "Rate limited (attempt {}/{}), retrying in {:?}",
                        attempt + 1,
                        config.max_retries,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
            Err(_elapsed) => {
                tracing::warn!(
                    "Provider chat connect timed out after {:?} (attempt {}/{})",
                    CHAT_CONNECT_TIMEOUT,
                    attempt + 1,
                    config.max_retries,
                );
                last_error = ProviderError::ApiError(format!(
                    "chat connect timed out after {:?}",
                    CHAT_CONNECT_TIMEOUT
                ));
            }
        }
    }

    Err(last_error)
}

/// Process a single LLM event and update the assistant message accordingly.
/// Returns Some(()) to continue, None to break the stream.
pub(crate) async fn process_stream_event<M: TryFrom<AgentEvent> + Send + 'static>(
    event: LlmEvent,
    assistant_message: &mut AgentMessage,
    pending_tool_calls: &mut HashMap<(String, String), PartialToolCall>,
    text_content: &mut String,
    text_buffer: &mut String,
    thinking_buffer: &mut String,
    turn: usize,
    msg_tx: &mpsc::Sender<M>,
    last_emit: &mut Instant,
) {
    const EMIT_DEBOUNCE_MS: u64 = 100;

    match event {
        LlmEvent::MessageStart { .. } => {
            send_event(msg_tx, AgentEvent::MessageStart {
                message: crate::events::AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![crate::events::ContentPart::Text { text: String::new() }],
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                    tool_calls: vec![],
                },
                turn,
            }).await;
        }
        LlmEvent::MessageDelta { content } => {
            text_buffer.push_str(&content);
            text_content.push_str(&content);

            // Send the NEW delta (append mode for live streaming).
            let delta = std::mem::take(text_buffer);
            assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
            let delta_len = delta.len();
            send_event(msg_tx, AgentEvent::MessageUpdate {
                message: assistant_message.clone(),
                delta,
                replace: false,
                turn,
            }).await;
            tracing::debug!("[ACTOR:AgentLoop] MessageUpdate delta (+{} chars, total={})", delta_len, text_content.len());
            *last_emit = Instant::now();
        }
        LlmEvent::ThinkingDelta { content } => {
            // Emit ThinkingStart when first ThinkingDelta arrives
            if thinking_buffer.is_empty() {
                send_event(msg_tx, AgentEvent::ThinkingStart { turn }).await;
            }
            thinking_buffer.push_str(&content);
            // Send only the NEW delta, not the full accumulated buffer.
            send_event(msg_tx, AgentEvent::ThinkingUpdate {
                delta: content,
                total_len: thinking_buffer.len(),
                turn,
            }).await;
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
                let _delta = std::mem::take(text_buffer);
                assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                send_event(msg_tx, AgentEvent::MessageUpdate {
                    message: assistant_message.clone(),
                    delta: String::new(),
                    replace: false,
                    turn,
                }).await;
            }
        }
        LlmEvent::Error { message } => {
            tracing::error!("[ACTOR:AgentLoop] Error: {}", message);
            assistant_message.error_message = Some(message);
        }
        LlmEvent::Usage { prompt_tokens, completion_tokens, total_tokens: _ } => {
            send_event(msg_tx, AgentEvent::TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                context_window: 128_000,
            }).await;
            tracing::debug!("[ACTOR:AgentLoop] Usage: {} prompt, {} completion tokens", prompt_tokens, completion_tokens);
        }
        LlmEvent::ToolExecutionStart { tool_call_id, tool_name, args, .. } => {
            let tool_args_str = args.to_string();
            send_event(msg_tx, AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                tool_args: tool_args_str,
                turn,
            }).await;
        }
        LlmEvent::ToolExecutionEnd { tool_call_id, result, .. } => {
            let content_text = result.content.clone();
            let tcid = tool_call_id.clone();
            send_event(msg_tx, AgentEvent::ToolExecutionEnd {
                tool_call_id: tcid.clone(),
                tool_name: String::new(), // Will be filled by handler
                tool_args: String::new(), // Will be filled by handler
                result: crate::events::ToolResult {
                    tool_call_id: tcid,
                    tool_name: String::new(),
                    input: serde_json::Value::Null,
                    content: vec![crate::events::ContentPart::Text { text: content_text }],
                    is_error: false,
                },
                duration_ms: 0,
                turn,
            }).await;
        }
        _ => {
            tracing::warn!("Unhandled LLM event variant in agent loop");
        }
    }
}

/// Send an agent event through the unified message channel.
pub(crate) async fn send_event<M: TryFrom<AgentEvent> + Send + 'static>(msg_tx: &mpsc::Sender<M>, event: AgentEvent) {
    if let Ok(msg) = M::try_from(event) {
        if msg_tx.send(msg).await.is_err() {
            tracing::error!("CRITICAL: Failed to send agent event - channel closed. User will not receive feedback!");
        }
    }
}

/// Finalize pending tool calls by converting them to AgentMessage content and tool_calls.
pub(crate) fn finalize_tool_calls(
    assistant_message: &mut AgentMessage,
    pending_tool_calls: &HashMap<(String, String), PartialToolCall>,
) {
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
}
