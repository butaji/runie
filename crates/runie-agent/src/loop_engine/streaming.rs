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

/// Accumulates streaming tool call deltas until MessageEnd
pub(crate) struct PartialToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Start chat with retry logic for rate limit errors.
/// Returns the stream on success, or the final error after retries are exhausted.
/// Non-rate-limit errors (like 401) fail immediately without retry.
pub(crate) async fn start_chat_with_retry(
    provider: Arc<dyn Provider>,
    messages: Vec<runie_core::Message>,
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

/// Process a single LLM event and update the assistant message accordingly.
/// Returns Some(()) to continue, None to break the stream.
pub(crate) async fn process_stream_event<M: TryFrom<AgentEvent> + Send + 'static>(
    event: LlmEvent,
    assistant_message: &mut AgentMessage,
    pending_tool_calls: &mut HashMap<(String, String), PartialToolCall>,
    text_content: &mut String,
    text_buffer: &mut String,
    turn: usize,
    msg_tx: &mpsc::Sender<M>,
    last_emit: &mut Instant,
) {
    const EMIT_DEBOUNCE_MS: u64 = 100;

    match event {
        LlmEvent::MessageDelta { content } => {
            text_buffer.push_str(&content);
            text_content.push_str(&content);

            let should_emit = text_buffer.contains('\n')
                || last_emit.elapsed().as_millis() > EMIT_DEBOUNCE_MS as u128;

            if should_emit {
                let delta = std::mem::take(text_buffer);
                assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                let delta_len = delta.len();
                send_event(msg_tx, AgentEvent::MessageUpdate {
                    message: assistant_message.clone(),
                    turn,
                    delta,
                }).await;
                tracing::debug!("[ACTOR:AgentLoop] MessageUpdate: \"{}\" (+{} chars)", &text_content[..text_content.len().saturating_sub(delta_len).min(50)], delta_len);
                *last_emit = Instant::now();
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
                let delta = std::mem::take(text_buffer);
                assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                send_event(msg_tx, AgentEvent::MessageUpdate {
                    message: assistant_message.clone(),
                    turn,
                    delta,
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
        _ => {
            tracing::warn!("Unhandled LLM event variant in agent loop");
        }
    }
}

/// Send an agent event through the unified message channel.
pub(crate) async fn send_event<M: TryFrom<AgentEvent> + Send + 'static>(msg_tx: &mpsc::Sender<M>, event: AgentEvent) {
    if let Ok(msg) = M::try_from(event) {
        if msg_tx.send(msg).await.is_err() {
            tracing::error!("Failed to send agent event");
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
