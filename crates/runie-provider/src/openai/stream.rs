//! OpenAI Chat Completions SSE streaming parser.
//!
//! Uses the `ProviderProtocol` trait to handle SSE frames.


use super::protocol::{OpenAiFrame, OpenAiProtocol, OpenAiState};
use super::request::build_request_body;
use super::types::ErrorBodyJson;
use super::OpenAiProvider;
use crate::protocol::ProviderProtocol;
use backon::{ExponentialBuilder, Retryable};
use reqwest_eventsource::retry::Never;
use reqwest_eventsource::EventSource;
use runie_core::proto::message::ChatMessage;
use runie_core::provider_event::ProviderEvent;
use tracing::Instrument;

/// Re-export types for testing and external consumers.
pub use super::protocol::ToolAccum;

/// Parse one SSE data-line into either a protocol frame or an error.
/// Handles regular SSE frames ("data: {...}") and fixture error lines ("error: {...}").
fn parse_sse_line(line: &str) -> Option<Result<OpenAiFrame, runie_core::provider_event::ModelError>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Support fixture error lines: "error: {\"type\":\"rate_limit\",...}"
    if let Some(err_json) = trimmed.strip_prefix("error: ") {
        if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(err_json) {
            tracing::trace!(line = %trimmed, "parsed SSE error line");
            return Some(Err(parse_error_value(&err_val)));
        }
    }
    let frame = OpenAiFrame::from_line(trimmed);
    if frame.is_some() {
        tracing::trace!(line = %trimmed, "parsed SSE frame");
    }
    frame.map(Ok)
}



pub fn openai_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send>> {
    Box::pin(openai_event_stream(provider, messages))
}

fn openai_event_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> impl futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send {
    let _span = tracing::info_span!(
        "openai_stream",
        provider = %provider.model(),
        base_url = %provider.base_url,
        message_count = %messages.len()
    );
    async_stream::stream! {
        tracing::debug!("starting OpenAI stream");
        // Build EventSource with backon retry for stream-establishment failures.
        // Once SSE data starts flowing, errors surface immediately (no internal retry).
        let es = match build_eventsource_with_retry(&provider, &messages).await {
            Ok(es) => {
                tracing::debug!("EventSource established");
                es
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to establish EventSource");
                yield Err(e); return;
            }
        };
        let mut es = Box::pin(es);
        let state = stream_sse_events(&mut es).await;
        tracing::debug!("SSE stream completed");
        for event in OpenAiProtocol::new().on_halt(state) { yield Ok(event); }
    }
}

/// Build an EventSource with `backon` retry for stream-establishment failures.
/// Once the SSE starts emitting data, errors surface immediately.
async fn build_eventsource_with_retry(
    provider: &OpenAiProvider,
    messages: &[ChatMessage],
) -> anyhow::Result<EventSource> {
    let span = tracing::debug_span!("build_eventsource");
    async move {
        let body = build_request_body(provider, messages);
        let url = format!("{}/chat/completions", provider.base_url);
        let api_key = provider.api_key.clone();
        let client = provider.client.clone();

        tracing::trace!(url = %url, body_size = %body.to_string().len(), "building request");

        // Expose secret only at the HTTP boundary
        let builder = client
            .post(&url)
            .header("Authorization", crate::http::bearer_header_secret(&api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        // Wrap EventSource creation in backon: retry on transient errors,
        // but only during stream establishment (before SSE data starts).
        let retry_config = provider.retry_config().cloned().unwrap_or_default();
        tracing::debug!(max_attempts = %retry_config.max_attempts, initial_delay_ms = %retry_config.initial_delay.as_millis(), "building EventSource with retry");
        // max_attempts includes the initial call, so max_times (retries) = max_attempts - 1.
        // Use saturating_sub to handle max_attempts = 0 or 1 (no retries).
        let backoff = ExponentialBuilder::default()
            .with_max_times(retry_config.max_attempts.saturating_sub(1) as usize)
            .with_min_delay(retry_config.initial_delay)
            .with_max_delay(retry_config.max_delay)
            .with_factor(retry_config.multiplier as f32);
        let es = (move || {
            // `RequestBuilder::try_clone()` returns `None` when the body is
            // non-repeatable (e.g., a streaming body). Since we use JSON bodies
            // (from `.json(&body)`), cloning always succeeds in practice.
            // If it fails, treat it as a fatal error (not retryable).
            let b = builder.try_clone().unwrap();
            async move {
                tracing::trace!("creating EventSource");
                let mut es = EventSource::new(b)
                    .map_err(|e| anyhow::anyhow!("EventSource build error: {e}"))?;
                // Disable internal retry: backon handles stream-establishment retries,
                // and we surface SSE-streaming errors immediately.
                es.set_retry_policy(Box::new(Never));
                Ok(es)
            }
        })
        .retry(backoff)
        .when(crate::retry::is_retryable)
        .await
        .inspect_err(|e| tracing::warn!(error = %e, "EventSource build failed after retries"))?;

        tracing::debug!("EventSource created successfully");
        Ok(es)
    }
    .instrument(span)
    .await
}

/// Process SSE events and yield provider events.
async fn stream_sse_events(
    es: &mut (impl futures::Stream<Item = Result<reqwest_eventsource::Event, reqwest_eventsource::Error>>
              + Unpin),
) -> OpenAiState {
    let span = tracing::debug_span!("stream_sse_events");
    async move {
        let protocol = OpenAiProtocol::new();
        let mut state = OpenAiState::default();
        let mut event_count = 0usize;

        while let Some(result) = futures::StreamExt::next(&mut *es).await {
            let line = match parse_sse_result(result) {
                Some(Ok(l)) => l,
                Some(Err(e)) => {
                    tracing::warn!(error = %e, "SSE stream error");
                    return state;
                }
                None => continue,
            };
            // Feed through the shared SSE parser (handles frames and fixture errors).
            match parse_sse_line(&line) {
                Some(Ok(frame)) => {
                    event_count += 1;
                    tracing::trace!(event_count = event_count, "processing SSE frame");
                    if protocol.terminal(&frame) {
                        let (new_state, _) = protocol.step(state, frame);
                        tracing::debug!(total_events = event_count, "SSE stream terminated");
                        return new_state;
                    }
                    let (new_state, _) = protocol.step(state, frame);
                    state = new_state;
                }
                Some(Err(err)) => {
                    tracing::warn!(error = %err, "SSE parse error");
                    // Error lines are fixture-only; in live streaming, SSE errors
                    // surface as reqwest_eventsource errors above, not as parsed lines.
                    continue;
                }
                None => continue,
            }
        }
        tracing::debug!(total_events = event_count, "SSE stream ended");
        state
    }
    .instrument(span)
    .await
}

fn parse_sse_result(
    result: Result<reqwest_eventsource::Event, reqwest_eventsource::Error>,
) -> Option<anyhow::Result<String>> {
    match result {
        Ok(reqwest_eventsource::Event::Open) => None,
        Ok(reqwest_eventsource::Event::Message(msg)) => Some(Ok(msg.data)),
        Err(e) => Some(Err(anyhow::anyhow!("{:?}", crate::retry::from_sse_error(&e)))),
    }
}

/// Replay SSE text and return accumulated events.
pub fn replay_sse(text: &str) -> Vec<ProviderEvent> {
    let protocol = OpenAiProtocol::new();
    let mut state = OpenAiState::default();
    let mut events = Vec::new();

    for line in text.lines() {
        match parse_sse_line(line) {
            Some(Ok(frame)) => {
                if protocol.terminal(&frame) {
                    let (_new_state, new_events) = protocol.step(std::mem::take(&mut state), frame);
                    events.extend(new_events);
                    break;
                }
                let (new_state, new_events) = protocol.step(std::mem::take(&mut state), frame);
                state = new_state;
                events.extend(new_events);
            }
            Some(Err(err)) => events.push(ProviderEvent::Error(err)),
            None => continue,
        }
    }
    events.extend(protocol.on_halt(state));
    events
}

/// Parse an SSE error line value into a ModelError.
fn parse_error_value(val: &serde_json::Value) -> runie_core::provider_event::ModelError {
    use runie_core::provider_event::ModelError;
    let err: ErrorBodyJson = match serde_json::from_value(val.clone()) {
        Ok(e) => e,
        Err(_) => {
            // Fallback: try to extract a message manually
            let msg = val
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return ModelError::Other(msg.to_string());
        }
    };

    let msg = err.message();
    let code = err.code();
    let type_ = err.type_();

    if code.contains("rate_limit") || type_.contains("rate_limit") {
        return ModelError::RateLimit {
            retry_after_secs: err.retry_after_secs(),
        };
    }
    if code.contains("context_length") || code.contains("token_limit") {
        return ModelError::ContextLength {
            limit: 0,
            used: 0,
        };
    }
    if code.contains("content_filter") || code.contains("refusal") || type_.contains("content_filter") {
        return ModelError::Refusal(msg.to_string());
    }
    ModelError::Other(msg.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn collect_events(lines: &[&str]) -> Vec<ProviderEvent> {
        let protocol = OpenAiProtocol::new();
        let mut state = OpenAiState::default();
        let mut all = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(frame) = OpenAiFrame::from_line(trimmed) {
                if protocol.terminal(&frame) {
                    let (_, events) = protocol.step(std::mem::take(&mut state), frame);
                    all.extend(events);
                    break;
                }
                let (new_state, events) = protocol.step(std::mem::take(&mut state), frame);
                state = new_state;
                all.extend(events);
            }
        }
        all.extend(protocol.on_halt(state));
        all
    }

    #[test]
    fn text_stream_emits_text_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"content\":\" World\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::TextDelta(_)))
            .expect("Should have TextDelta");
        assert!(
            matches!(&events[0], ProviderEvent::TextStart { id } if id == "text"),
            "First event should be TextStart"
        );
        let start_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::TextStart { .. }))
            .expect("Should have TextStart");
        assert!(start_idx < first_delta_idx);
        let text_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProviderEvent::TextStart { id } if id == "text"))
            .collect();
        assert_eq!(text_starts.len(), 1);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProviderEvent::Finish { .. })),
            "Should emit Finish"
        );
    }

    #[test]
    fn reasoning_stream_emits_thinking_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\" more\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::ThinkingDelta(_)))
            .expect("Should have ThinkingDelta");
        assert!(
            matches!(&events[0], ProviderEvent::ThinkingStart { id } if id == "reasoning"),
            "First event should be ThinkingStart"
        );
        let start_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .expect("Should have ThinkingStart");
        assert!(start_idx < first_delta_idx);
        let thinking_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingStart { id } if id == "reasoning"))
            .collect();
        assert_eq!(thinking_starts.len(), 1);
    }

    /// Layer 4: OpenAI SSE stream accumulates to canonical ToolCall.
    #[test]
    fn openai_stream_accumulates_canonical_tool_calls() {
        let lines = &[
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"read_file","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\"C"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"argo.toml\"}"}}]}}]}"#,
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        // Find ToolCallEnd events and verify they contain canonical ToolCalls
        let tool_ends: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ProviderEvent::ToolCallEnd { id } = e {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            tool_ends,
            vec!["call_abc"],
            "Should emit ToolCallEnd with canonical id"
        );
        // Verify ToolCallStart was emitted with the name
        let tool_starts: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ProviderEvent::ToolCallStart { id, name } = e {
                    Some((id.clone(), name.clone()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tool_starts, vec![("call_abc".into(), "read_file".into())]);
    }
}
