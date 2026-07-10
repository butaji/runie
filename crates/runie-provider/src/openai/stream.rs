//! OpenAI Chat Completions SSE streaming parser.
//!
//! Uses the `ProviderProtocol` trait to handle SSE frames.

use super::protocol::{OpenAiFrame, OpenAiProtocol, OpenAiState};
use super::request::build_request_body;
use super::types::ErrorBodyJson;
use super::OpenAiProvider;
use crate::protocol::ProviderProtocol;
use reqwest_eventsource::retry::Never;
use reqwest_eventsource::EventSource;
use runie_core::proto::message::ChatMessage;
use runie_core::provider_event::ProviderEvent;
use tracing::Instrument;

/// Re-export types for testing and external consumers.
pub use super::protocol::ToolAccum;

/// Default per-read idle timeout for the SSE stream. If no data arrives within
/// this window, the stream is aborted and surfaced as an error rather than
/// hanging until the (much longer) total request timeout.
pub const DEFAULT_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Parse one SSE data-line into either a protocol frame or an error.
/// Handles regular SSE frames ("data: {...}") and fixture error lines ("error: {...}").
fn parse_sse_line(
    line: &str,
) -> Option<Result<OpenAiFrame, runie_core::provider_event::ModelError>> {
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
        let retry_config = provider.retry_config().cloned().unwrap_or_default();
        let idle_timeout = provider.idle_timeout().unwrap_or(DEFAULT_IDLE_TIMEOUT);

        // Whole-request retry: each attempt builds a fresh EventSource and
        // consumes the SSE stream to completion. Content is only yielded to the
        // consumer after a fully successful attempt, so retrying a failed
        // attempt (transient 5xx/429/transport before any content) never
        // duplicates output. Once content has streamed, a later failure is
        // surfaced as an error rather than retried.
        let (state, streamed_events) = match crate::retry::with_retry_config(
            || async {
                let mut es = build_eventsource(&provider, &messages).await?;
                stream_sse_events(&mut es, idle_timeout).await
            },
            &retry_config,
        )
        .await
        {
            Ok(pair) => pair,
            Err(e) => {
                // A real transport/HTTP failure (possibly after exhausting
                // retries). Surface it so the UI/CLI shows an error instead of
                // an empty "successful" turn. Do NOT emit `Finish`.
                tracing::warn!(error = %e, "OpenAI stream failed");
                yield Err(e);
                return;
            }
        };
        tracing::debug!("SSE stream completed with state");
        // Yield every event produced while parsing SSE frames first. These must
        // be preserved even when the stream ends without `data: [DONE]` (e.g.
        // minimax closing the connection after the last chunk).
        for event in streamed_events {
            tracing::debug!(event = ?event, "yielding event");
            yield Ok(event);
        }
        let halt_events = OpenAiProtocol::new().on_halt(state);
        tracing::debug!(events_count = halt_events.len(), "on_halt events");
        for event in halt_events {
            tracing::debug!(event = ?event, "yielding event");
            yield Ok(event);
        }
    }
}

/// Build a single-attempt `EventSource` (no internal retry).
///
/// Whole-request retry is handled by the caller via
/// `crate::retry::with_retry_config`, which builds a *fresh* EventSource per
/// attempt. Once SSE data starts flowing, errors surface immediately.
async fn build_eventsource(
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

        // `RequestBuilder::try_clone()` returns `None` only for non-repeatable
        // (streaming) bodies; our JSON body is repeatable, so this is infallible.
        let builder = builder.try_clone().unwrap();

        tracing::trace!("creating EventSource");
        let mut es = EventSource::new(builder)
            .map_err(|e| anyhow::anyhow!("EventSource build error: {e}"))?;
        // Disable reqwest-eventsource's internal retry: whole-request retry is
        // handled by the caller, and SSE-streaming errors surface immediately.
        es.set_retry_policy(Box::new(Never));
        Ok(es)
    }
    .instrument(span)
    .await
}

/// Process SSE events and return the accumulated protocol state plus every
/// provider event produced while parsing frames. Returning the events (rather
/// than dropping them) ensures content survives a stream that ends without a
/// terminal `data: [DONE]` frame.
async fn stream_sse_events(
    es: &mut (impl futures::Stream<Item = Result<reqwest_eventsource::Event, reqwest_eventsource::Error>>
              + Unpin),
    idle_timeout: std::time::Duration,
) -> anyhow::Result<(OpenAiState, Vec<ProviderEvent>)> {
    let span = tracing::debug_span!("stream_sse_events");
    async move {
        let protocol = OpenAiProtocol::new();
        let mut state = OpenAiState::default();
        let mut events = Vec::new();
        let mut event_count = 0usize;

        loop {
            let result = match tokio::time::timeout(
                idle_timeout,
                futures::StreamExt::next(&mut *es),
            )
            .await
            {
                Ok(Some(r)) => r,
                Ok(None) => break, // underlying stream ended
                Err(_elapsed) => {
                    // NOTE: the message intentionally avoids retry-heuristic
                    // tokens ("timeout", "connection", ...) in retry.rs so a
                    // stalled stream is surfaced after one idle window rather
                    // than being retried (which would multiply the hang).
                    tracing::warn!(?idle_timeout, "SSE stream stalled (no data received)");
                    return Err(anyhow::anyhow!(
                        "SSE stream stalled: no data received for {idle_timeout:?}"
                    ));
                }
            };
            let line = match result {
                Ok(reqwest_eventsource::Event::Open) => continue,
                Ok(reqwest_eventsource::Event::Message(msg)) => msg.data,
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    // Clean EOF without `data: [DONE]` (e.g. minimax). Not an
                    // error: preserve whatever content we already produced.
                    tracing::debug!("SSE stream ended cleanly (no [DONE])");
                    break;
                }
                Err(e) => {
                    // Non-200 status, DNS/connect/TLS failure, mid-stream reset,
                    // or timeout.
                    if event_count > 0 {
                        // The stream already emitted content and then dropped
                        // (e.g. minimax closing the connection after the last
                        // chunk). Treat as a benign truncation: preserve what
                        // we streamed and finish normally rather than retrying
                        // (which would hit a now-dead endpoint and lose the
                        // content) or surfacing a spurious error.
                        tracing::warn!(
                            error = ?crate::retry::from_sse_error(&e),
                            event_count,
                            "SSE stream dropped after emitting content; treating as truncation"
                        );
                        break;
                    }
                    // No content yet: a genuine failure. Return a *typed*
                    // ProviderError so callers (and the retry wrapper) can
                    // classify it via downcast.
                    tracing::warn!(error = ?crate::retry::from_sse_error(&e), "SSE stream error");
                    return Err(crate::retry::from_sse_error(&e).into());
                }
            };
            // Feed through the shared SSE parser (handles frames and fixture errors).
            match parse_sse_line(&line) {
                Some(Ok(frame)) => {
                    event_count += 1;
                    tracing::trace!(event_count = event_count, "processing SSE frame");
                    if protocol.terminal(&frame) {
                        let (new_state, frame_events) = protocol.step(state, frame);
                        tracing::debug!(events = ?frame_events, "terminal frame events");
                        events.extend(frame_events);
                        tracing::debug!(total_events = event_count, "SSE stream terminated");
                        return Ok((new_state, events));
                    }
                    let (new_state, frame_events) = protocol.step(state, frame);
                    tracing::debug!(events = ?frame_events, "chunk events");
                    events.extend(frame_events);
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
        Ok((state, events))
    }
    .instrument(span)
    .await
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
        return ModelError::ContextLength { limit: 0, used: 0 };
    }
    if code.contains("content_filter")
        || code.contains("refusal")
        || type_.contains("content_filter")
    {
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

    /// Build an `Ok(Event::Message)` item carrying the given SSE data line.
    fn sse_message(data: &str) -> Result<reqwest_eventsource::Event, reqwest_eventsource::Error> {
        let mut event = reqwest_eventsource::Event::Message(Default::default());
        if let reqwest_eventsource::Event::Message(ref mut m) = event {
            m.data = data.to_string();
        }
        Ok(event)
    }

    /// Regression test: when the SSE stream ends unexpectedly (no `data: [DONE]`,
    /// e.g. minimax closing the connection after the last chunk), the content
    /// already streamed MUST still be yielded. Previously, `stream_sse_events`
    /// discarded the events returned by `protocol.step()` and only the final
    /// `on_halt` events (just `Finish`) survived, so the assistant's text was
    /// lost.
    #[tokio::test]
    async fn stream_sse_events_yields_content_when_stream_ends_without_done() {
        let items: Vec<
            Result<reqwest_eventsource::Event, reqwest_eventsource::Error>,
        > = vec![
            sse_message(r#"{"choices":[{"delta":{"content":"Hello"}}]}"#),
            sse_message(r#"{"choices":[{"delta":{"content":" world"}}]}"#),
            // Stream closes without a terminal `data: [DONE]` frame.
            Err(reqwest_eventsource::Error::StreamEnded),
        ];
        let mut stream = futures::stream::iter(items);

        let (state, streamed_events) = stream_sse_events(
            &mut stream,
            std::time::Duration::from_secs(60),
        )
        .await
        .expect("a clean StreamEnded (no [DONE]) must not be an error");
        let mut all = streamed_events;
        all.extend(OpenAiProtocol::new().on_halt(state));

        assert!(
            all.iter()
                .any(|e| matches!(e, ProviderEvent::TextDelta(d) if d == "Hello")),
            "expected TextDelta(\"Hello\") to survive the truncated stream, got: {all:?}"
        );
        assert!(
            all.iter()
                .any(|e| matches!(e, ProviderEvent::TextDelta(d) if d == " world")),
            "expected TextDelta(\" world\") to survive the truncated stream, got: {all:?}"
        );
        assert!(
            all.iter().any(|e| matches!(e, ProviderEvent::Finish { .. })),
            "expected a Finish event to terminate the stream, got: {all:?}"
        );
    }

    /// Regression: a non-200 / transport failure from the live SSE endpoint MUST
    /// surface as an `Err` item from the provider stream (so the UI/CLI shows an
    /// error), and MUST NOT be silently converted into a normal `Finish`.
    ///
    /// Previously `stream_sse_events` logged the `reqwest_eventsource` error and
    /// returned early, after which `on_halt` unconditionally appended `Finish` —
    /// so connection refused / DNS / TLS / HTTP 500 / mid-stream drop all looked
    /// like an empty-but-successful turn.
    #[tokio::test]
    async fn openai_stream_surfaces_http_errors_instead_of_silent_finish() {
        use futures::StreamExt;
        use runie_core::proto::message::ChatMessage;
        use runie_core::Provider as _;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&mock_server)
            .await;

        let provider = OpenAiProvider::new("sk-test".to_string(), "gpt-4o")
            .with_base_url(mock_server.uri());
        let mut stream = provider.generate(vec![ChatMessage::user("hi".to_string())]);

        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }

        assert!(
            items.iter().any(|i| i.is_err()),
            "expected the provider stream to yield an Err for HTTP 500, got all-Ok: {items:?}"
        );
        assert!(
            !items
                .iter()
                .any(|i| matches!(i, Ok(ProviderEvent::Finish { .. }))),
            "a failed stream must not be reported as a normal Finish, got: {items:?}"
        );
    }

    /// Regression: an SSE stream that opens but then stops sending data MUST be
    /// cut off by a per-read idle timeout (surfaced as an `Err`), not hang until
    /// the 120 s total request timeout fires and then get reported as success.
    #[tokio::test]
    async fn openai_stream_idle_timeout_surfaces_error_on_stalled_stream() {
        use futures::StreamExt;
        use runie_core::proto::message::ChatMessage;
        use runie_core::Provider as _;
        use std::time::{Duration, Instant};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_delay(Duration::from_secs(3)),
            )
            .mount(&mock_server)
            .await;

        let provider = OpenAiProvider::new("sk-test".to_string(), "gpt-4o")
            .with_base_url(mock_server.uri())
            .with_idle_timeout(Duration::from_millis(150));

        let start = Instant::now();
        let mut stream = provider.generate(vec![ChatMessage::user("hi".to_string())]);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        let elapsed = start.elapsed();

        assert!(
            items.iter().any(|i| i.is_err()),
            "expected an idle-timeout Err for a stalled stream, got: {items:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "idle timeout should fire well before the server's 3s delay, took {elapsed:?}"
        );
    }

    /// Regression: transient failures (e.g. HTTP 503) MUST be retried at the
    /// whole-request level so a later success is delivered to the consumer.
    /// Previously the only retry wrapped `EventSource::new` construction, which
    /// cannot fail for network reasons, so a transient 503 was surfaced as an
    /// immediate error with no retry.
    #[tokio::test]
    async fn openai_stream_retries_transient_http_then_succeeds() {
        use futures::StreamExt;
        use runie_core::proto::message::ChatMessage;
        use runie_core::Provider as _;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        // Raw-TCP mock: first two requests get a retryable 503, the third a
        // 200 SSE body. Raw TCP avoids wiremock content-type fragility so the
        // 503 is classified as a status error (retryable), not a content error.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_for = hits.clone();
        tokio::spawn(async move {
            for _ in 0..3 {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                let n = hits_for.fetch_add(1, Ordering::SeqCst) + 1;
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                loop {
                    match sock.read(&mut tmp).await {
                        Ok(0) | Err(_) => break,
                        Ok(k) => {
                            buf.extend_from_slice(&tmp[..k]);
                            if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 64 * 1024 {
                                break;
                            }
                        }
                    }
                }
                let resp = if n <= 2 {
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nretry\n"
                        .to_string()
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n\
                     data: {\"choices\":[{\"delta\":{\"content\":\"pong\"}}]}\n\n\
                     data: [DONE]\n\n"
                        .to_string()
                };
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.flush().await;
            }
        });

        let provider = OpenAiProvider::new("sk-test".to_string(), "gpt-4o")
            .with_base_url(base_url)
            .with_retry_config(crate::RetryConfig::new(
                3,
                Duration::from_millis(1),
                Duration::from_millis(20),
                1.0,
            ));

        let mut stream = provider.generate(vec![ChatMessage::user("hi".to_string())]);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }

        assert_eq!(
            hits.load(Ordering::SeqCst),
            3,
            "expected exactly 3 attempts (2 transient failures + 1 success)"
        );
        assert!(
            items.iter().all(|i| i.is_ok()),
            "expected success after retrying transient 503s, got an error: {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|i| matches!(i, Ok(ProviderEvent::TextDelta(d)) if d == "pong")),
            "expected retried request to deliver TextDelta(\"pong\"), got: {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|i| matches!(i, Ok(ProviderEvent::Finish { .. }))),
            "expected Finish after successful retry, got: {items:?}"
        );
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
