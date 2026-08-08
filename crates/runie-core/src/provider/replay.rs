//! Small deterministic replay provider for recorded SSE traces.

use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::types::{
    AssistantMessage, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, ToolCall,
    Usage,
};
use futures::stream;

use super::{
    http::HttpActor,
    stream_fn::{AssistantMessageEventStream, StreamError, StreamFn},
};

/// Replays text/reasoning events from an OpenAI-, Anthropic-, or Gemini-style
/// SSE capture. Provider-specific transport details stay in the fixture; the
/// core only receives its normal `AssistantMessageEvent` stream.
pub struct ReplayProvider {
    events: Vec<AssistantMessageEvent>,
    deferred_events: Option<Vec<AssistantMessageEvent>>,
    /// Number of `stream()` calls. The first call replays the recorded trace;
    /// later calls (auto-continue after a tool batch) return a terminating
    /// `Done{stop}` so the loop does not re-replay the same trace forever.
    calls: AtomicUsize,
}

impl ReplayProvider {
    pub async fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let input = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| StreamError::Invalid(e.to_string()))?;
        Self::from_sse_body(&input)
    }

    pub async fn from_http(http: Arc<dyn HttpActor>) -> Result<Self, StreamError> {
        Self::from_http_with_options(http, Model::default(), None).await
    }

    pub async fn from_http_with_options(
        http: Arc<dyn HttpActor>,
        model: Model,
        options: Option<SimpleStreamOptions>,
    ) -> Result<Self, StreamError> {
        let response = http
            .post_with_options(String::new(), model, options)
            .await?;
        if response.status >= 400 {
            return Err(StreamError::Api(format!("HTTP {}", response.status)));
        }
        Self::from_sse_body(&response.body)
    }

    /// Build a deterministic provider from Codex Responses WebSocket text
    /// messages. Socket acquisition, continuation caching, fallback, and
    /// cleanup remain owned by the concrete WebSocket adapter; this entry
    /// point only reuses the source-aligned Responses event decoder.
    pub fn from_websocket_messages<I, S>(messages: I) -> Result<Self, StreamError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let input = messages
            .into_iter()
            .map(|message| {
                let text = message.as_ref();
                let value = serde_json::from_str::<serde_json::Value>(text).map_err(|error| {
                    StreamError::Invalid(format!("invalid Codex WebSocket JSON: {error}"))
                })?;
                if !value.is_object() {
                    return Err(StreamError::Invalid(
                        "Codex WebSocket message must be a JSON object".into(),
                    ));
                }
                Ok(format!("data: {text}"))
            })
            .collect::<Result<Vec<_>, StreamError>>()?
            .join("\n");
        Self::from_sse_body(&input)
    }

    #[allow(clippy::too_many_lines)]
    fn from_sse_body(input: &str) -> Result<Self, StreamError> {
        let mut events = vec![AssistantMessageEvent::Start {
            partial: AssistantMessage::default(),
        }];
        let mut finished = false;
        let mut stop_reason = StopReason::Stop;
        let mut usage = Usage::default();
        let mut tool_calls = std::collections::BTreeMap::<usize, (String, String, String)>::new();
        for line in input.lines() {
            if let Some(raw_error) = line.strip_prefix("error:").map(str::trim_start) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw_error) {
                    return Err(StreamError::Api(value.to_string()));
                }
                return Err(StreamError::Api(raw_error.to_owned()));
            }
            let Some(raw) = line.strip_prefix("data:").map(str::trim_start) else {
                continue;
            };
            if raw == "[DONE]" {
                finished = true;
                break;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
                continue;
            };
            match value.get("type").and_then(|v| v.as_str()) {
                Some("error") | Some("response.failed") => {
                    return Err(StreamError::Api(response_error_message(&value)));
                }
                _ => {}
            }
            finished |= append_text_events(&value, &mut events);
            collect_tool_calls(&value, &mut tool_calls);
            if has_terminal_marker(&value) {
                finished = true;
                stop_reason = response_stop_reason(&value);
                usage = response_usage(&value);
            }
        }
        if !finished {
            return Err(StreamError::Invalid("trace has no terminal event".into()));
        }
        finish_replay_events(&mut events, tool_calls, stop_reason, usage);
        Ok(Self {
            events,
            deferred_events: None,
            calls: AtomicUsize::new(0),
        })
    }

    /// Attach a provider-scoped deferred result to a deterministic replay.
    /// The generic actor still owns command admission and pump cleanup; this
    /// capability only supplies the adapter's already-decoded event stream.
    pub fn with_deferred_events(mut self, events: Vec<AssistantMessageEvent>) -> Self {
        self.deferred_events = Some(events);
        self
    }
}

fn response_error_message(value: &serde_json::Value) -> String {
    let response = value.get("response").unwrap_or(value);
    let error = response.get("error");
    let code = error.and_then(|v| v.get("code")).and_then(|v| v.as_str());
    let message = error
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str());
    let reason = response
        .get("incomplete_details")
        .and_then(|v| v.get("reason"))
        .and_then(|v| v.as_str());
    match (code, message, reason) {
        (Some(code), Some(message), _) => format!("{code}: {message}"),
        (_, Some(message), _) => message.to_owned(),
        (_, _, Some(reason)) => format!("incomplete: {reason}"),
        _ => value.to_string(),
    }
}

fn response_stop_reason(value: &serde_json::Value) -> StopReason {
    match value.get("type").and_then(|v| v.as_str()) {
        Some("response.incomplete") => {
            let reason = value
                .pointer("/response/incomplete_details/reason")
                .and_then(|v| v.as_str());
            if reason == Some("max_output_tokens") {
                StopReason::MaxTokens
            } else {
                StopReason::Error
            }
        }
        // Keep the legacy chat-completions replay contract unchanged. Its
        // tool-call completion reason is finalized by the reconstructed
        // content path; Responses status mapping is handled above.
        _ => StopReason::Stop,
    }
}

fn response_usage(value: &serde_json::Value) -> Usage {
    let Some(raw) = value.pointer("/response/usage") else {
        return Usage::default();
    };
    let input = raw
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let output = raw
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let cache_read = raw
        .pointer("/input_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let reasoning = raw
        .pointer("/output_tokens_details/reasoning_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    Usage {
        input: input.saturating_sub(cache_read),
        output,
        cache_read,
        total_tokens: raw
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(input.saturating_add(output)),
        reasoning,
        ..Usage::default()
    }
}

fn finish_replay_events(
    events: &mut Vec<AssistantMessageEvent>,
    tool_calls: std::collections::BTreeMap<usize, (String, String, String)>,
    stop_reason: StopReason,
    usage: Usage,
) {
    for (_, (id, name, arguments)) in tool_calls {
        let args = serde_json::from_str(&arguments)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        events.push(AssistantMessageEvent::ToolCallDelta {
            index: 0,
            delta: arguments.clone(),
            partial: AssistantMessage::with_tool_call(ToolCall {
                id,
                name,
                arguments: args,
                thought_signature: None,
            }),
        });
    }
    events.push(AssistantMessageEvent::Done {
        stop_reason,
        usage,
        message: None,
    });
}

#[allow(clippy::too_many_lines)]
fn append_text_events(value: &serde_json::Value, events: &mut Vec<AssistantMessageEvent>) -> bool {
    for (pointer, thinking) in [
        ("/choices/0/delta/content", false),
        ("/delta/text", false),
        ("/choices/0/delta/reasoning_content", true),
        ("/delta/thinking", true),
        ("/delta/reasoning", true),
    ] {
        if let Some(text) = value
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            events.push(if thinking {
                AssistantMessageEvent::ThinkingDelta {
                    index: 1,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            } else {
                AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            });
        }
    }
    // OpenAI Responses/Codex emits typed events whose payload is
    // `{delta: ...}`, rather than the chat-completions delta shape.
    match value.get("type").and_then(|v| v.as_str()) {
        Some("response.output_text.delta") | Some("response.refusal.delta") => {
            if let Some(text) = value
                .get("delta")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                events.push(AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                });
            }
        }
        Some("response.reasoning_summary_text.delta") | Some("response.reasoning_text.delta") => {
            if let Some(text) = value
                .get("delta")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                events.push(AssistantMessageEvent::ThinkingDelta {
                    index: 1,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                });
            }
        }
        _ => {}
    }
    false
}

fn collect_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    collect_anthropic_tool_call(value, tool_calls);
    collect_openai_tool_calls(value, tool_calls);
    collect_responses_tool_call(value, tool_calls);
}

fn collect_anthropic_tool_call(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    if let Some(partial) = value
        .pointer("/delta/partial_json")
        .and_then(|v| v.as_str())
    {
        tool_calls.entry(0).or_default().2.push_str(partial);
    }
    let Some(block) = value
        .pointer("/content_block")
        .filter(|v| v.get("type").and_then(|x| x.as_str()) == Some("tool_use"))
    else {
        return;
    };
    let entry = tool_calls
        .entry(value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
        .or_default();
    entry.0 = block
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("trace-tool")
        .into();
    entry.1 = block
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .into();
}

fn collect_openai_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    let Some(calls) = value
        .pointer("/choices/0/delta/tool_calls")
        .and_then(|v| v.as_array())
    else {
        return;
    };
    for call in calls {
        let entry = tool_calls
            .entry(call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
            .or_default();
        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
            entry.0 = id.into();
        }
        if let Some(name) = call.pointer("/function/name").and_then(|v| v.as_str()) {
            entry.1 = name.into();
        }
        if let Some(args) = call.pointer("/function/arguments").and_then(|v| v.as_str()) {
            entry.2.push_str(args);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn collect_responses_tool_call(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    let Some(event_type) = value.get("type").and_then(|v| v.as_str()) else {
        return;
    };
    match event_type {
        "response.output_item.added" => {
            let Some(item) = value.get("item") else {
                return;
            };
            if item.get("type").and_then(|v| v.as_str()) != Some("function_call") {
                return;
            }
            let index = value
                .get("output_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let entry = tool_calls.entry(index).or_default();
            if let Some(id) = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str())
            {
                entry.0 = id.into();
            }
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                entry.1 = name.into();
            }
        }
        "response.function_call_arguments.delta" => {
            let index = value
                .get("output_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let entry = tool_calls.entry(index).or_default();
            if let Some(delta) = value.get("delta").and_then(|v| v.as_str()) {
                entry.2.push_str(delta);
            }
        }
        "response.function_call_arguments.done" => {
            let index = value
                .get("output_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let entry = tool_calls.entry(index).or_default();
            if let Some(arguments) = value.get("arguments").and_then(|v| v.as_str()) {
                entry.2 = arguments.into();
            }
        }
        "response.output_item.done" => {
            let Some(item) = value.get("item") else {
                return;
            };
            if item.get("type").and_then(|v| v.as_str()) != Some("function_call") {
                return;
            }
            let index = value
                .get("output_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let entry = tool_calls.entry(index).or_default();
            if let Some(id) = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str())
            {
                entry.0 = id.into();
            }
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                entry.1 = name.into();
            }
            if let Some(arguments) = item.get("arguments").and_then(|v| v.as_str()) {
                entry.2 = arguments.into();
            }
        }
        _ => {}
    }
}

fn has_terminal_marker(value: &serde_json::Value) -> bool {
    value.get("type").and_then(|v| v.as_str()) == Some("message_stop")
        || value.get("type").and_then(|v| v.as_str()) == Some("response.completed")
        || value.get("type").and_then(|v| v.as_str()) == Some("response.incomplete")
        || value
            .pointer("/delta/stop_reason")
            .is_some_and(|v| !v.is_null())
        || value
            .pointer("/choices/0/finish_reason")
            .is_some_and(|v| !v.is_null())
}

impl ReplayProvider {
    /// Reset the turn counter so a fresh run replays the recorded trace again.
    pub fn reset_turns(&self) {
        self.calls.store(0, Ordering::Release);
    }
}

#[async_trait::async_trait]
impl StreamFn for ReplayProvider {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let n = self.calls.fetch_add(1, Ordering::AcqRel) + 1;
        if n > 1 {
            return Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            }])));
        }
        Ok(Box::pin(stream::iter(self.events.clone())))
    }

    async fn fetch_deferred(
        &self,
        _model: &Model,
        handle: &crate::types::DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let Some(events) = self.deferred_events.as_ref() else {
            return Err(StreamError::Invalid(
                "replay provider has no deferred response fixture".into(),
            ));
        };
        if handle.id.is_empty() {
            return Err(StreamError::Invalid(
                "deferred response handle id is required".into(),
            ));
        }
        Ok(Box::pin(stream::iter(events.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AssistantContent;
    use futures::StreamExt;

    #[tokio::test]
    async fn responses_trace_maps_text_delta_and_completion_to_pi_events() {
        let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\
             data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\
             data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":7,\"total_tokens\":19,\"input_tokens_details\":{\"cached_tokens\":3},\"output_tokens_details\":{\"reasoning_tokens\":2}}}}\n",
        )
        .expect("Responses trace");
        let mut events = provider
            .stream(
                &Model::default(),
                &crate::types::AgentContext::default(),
                None,
            )
            .await
            .expect("replay stream");
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Start { .. })
        ));
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::TextDelta { delta, .. }) if delta == "hello"
        ));
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage,
                ..
            }) if usage.input == 9
                && usage.output == 7
                && usage.cache_read == 3
                && usage.reasoning == 2
                && usage.total_tokens == 19
        ));
    }

    #[tokio::test]
    async fn deferred_replay_uses_provider_scoped_event_fixture() {
        let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
        )
        .expect("ordinary replay trace")
        .with_deferred_events(vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }]);
        let mut events = provider
            .fetch_deferred(
                &Model::default(),
                &crate::types::DeferredHandle {
                    provider: "replay".into(),
                    model_id: "model".into(),
                    api: "responses".into(),
                    id: "deferred-1".into(),
                    expires_at: None,
                    poll_after_ms: None,
                    data: None,
                },
                None,
            )
            .await
            .expect("deferred replay capability");
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Done { .. })
        ));
    }

    #[tokio::test]
    async fn websocket_messages_use_the_same_codex_responses_decoder() {
        let provider = ReplayProvider::from_websocket_messages([
            r#"{"type":"response.created","response":{"id":"ws-1"}}"#,
            r#"{"type":"response.output_text.delta","delta":"socket"}"#,
            r#"{"type":"response.completed","response":{"status":"completed"}}"#,
        ])
        .expect("WebSocket Responses messages");
        let mut events = provider
            .stream(
                &Model::default(),
                &crate::types::AgentContext::default(),
                None,
            )
            .await
            .expect("replay stream");
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Start { .. })
        ));
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::TextDelta { delta, .. }) if delta == "socket"
        ));
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                ..
            })
        ));
    }

    #[test]
    fn websocket_decoder_rejects_malformed_or_non_object_messages() {
        let malformed = ReplayProvider::from_websocket_messages(["not-json"]);
        assert!(
            matches!(malformed, Err(StreamError::Invalid(message)) if message.contains("invalid Codex WebSocket JSON"))
        );

        let scalar = ReplayProvider::from_websocket_messages(["null"]);
        assert!(
            matches!(scalar, Err(StreamError::Invalid(message)) if message.contains("must be a JSON object"))
        );
    }

    #[tokio::test]
    async fn responses_trace_collects_function_call_arguments_by_output_index() {
        let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.output_item.added\",\"output_index\":2,\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"echo\"}}\n\
             data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":2,\"delta\":\"{\\\"x\\\":\"}\n\
             data: {\"type\":\"response.function_call_arguments.done\",\"output_index\":2,\"arguments\":\"{\\\"x\\\":1}\"}\n\
             data: {\"type\":\"response.output_item.done\",\"output_index\":2,\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"echo\",\"arguments\":\"{\\\"x\\\":1}\"}}\n\
             data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
        )
        .expect("Responses tool trace");
        let mut events = provider
            .stream(
                &Model::default(),
                &crate::types::AgentContext::default(),
                None,
            )
            .await
            .expect("replay stream");
        let mut tool_call = None;
        while let Some(event) = events.next().await {
            if let AssistantMessageEvent::ToolCallDelta { partial, .. } = event {
                tool_call = partial
                    .content
                    .into_iter()
                    .find_map(|content| match content {
                        AssistantContent::ToolCall(call) => Some(call),
                        _ => None,
                    });
            }
        }
        let tool_call = tool_call.expect("tool call event");
        assert_eq!(tool_call.id, "call-1");
        assert_eq!(tool_call.name, "echo");
        assert_eq!(tool_call.arguments, serde_json::json!({"x": 1}));
    }

    #[test]
    fn responses_failed_trace_preserves_provider_code_and_message() {
        let result = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"rate_limit\",\"message\":\"try later\"}}}\n",
        );
        let error = match result {
            Ok(_) => panic!("failed Responses trace must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            StreamError::Api(message) if message == "rate_limit: try later"
        ));
    }

    #[tokio::test]
    async fn responses_incomplete_max_output_maps_to_length() {
        let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.incomplete\",\"response\":{\"status\":\"incomplete\",\"incomplete_details\":{\"reason\":\"max_output_tokens\"}}}\n",
        )
        .expect("incomplete Responses trace");
        let mut events = provider
            .stream(
                &Model::default(),
                &crate::types::AgentContext::default(),
                None,
            )
            .await
            .expect("replay stream");
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Start { .. })
        ));
        assert!(matches!(
            events.next().await,
            Some(AssistantMessageEvent::Done {
                stop_reason: StopReason::MaxTokens,
                ..
            })
        ));
    }
}
