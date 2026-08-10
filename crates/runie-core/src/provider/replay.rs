//! Small deterministic replay provider for recorded SSE traces.

use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::types::{
    AssistantMessage, AssistantMessageEvent, DeferredHandle, Model, SimpleStreamOptions,
    StopReason, ToolCall, Usage,
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
    deferred_poll_events: std::sync::Mutex<Option<Vec<Vec<AssistantMessageEvent>>>>,
    deferred_handle: Option<DeferredHandle>,
    deferred_cancelled: std::sync::atomic::AtomicBool,
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
            return Err(StreamError::Api(http_error_message(
                response.status,
                &response.body,
            )));
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

    fn from_sse_body(input: &str) -> Result<Self, StreamError> {
        let mut events = vec![AssistantMessageEvent::Start {
            partial: AssistantMessage::default(),
        }];
        let mut state = SseParseState::default();
        for line in input.lines() {
            if consume_sse_line(line, &mut state, &mut events)? {
                break;
            }
        }
        if !state.finished {
            return Err(StreamError::Invalid("trace has no terminal event".into()));
        }
        finish_replay_events(
            &mut events,
            state.tool_calls,
            state.stop_reason,
            state.usage,
        );
        Ok(Self {
            events,
            deferred_events: None,
            deferred_poll_events: std::sync::Mutex::new(None),
            deferred_handle: None,
            deferred_cancelled: std::sync::atomic::AtomicBool::new(false),
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

    /// Bind the replayed deferred response to the provider-scoped identity
    /// Pi passes to an adapter. This keeps replay fixtures from accepting a
    /// handle belonging to another provider/model/API lane.
    pub fn with_deferred_handle(mut self, handle: DeferredHandle) -> Self {
        self.deferred_handle = Some(handle);
        self
    }

    /// Attach an ordered provider-owned polling sequence. Each deferred fetch
    /// consumes one event batch, allowing a fixture to model pending progress
    /// followed by a terminal result without introducing timing or sleeps.
    pub fn with_deferred_poll_events(self, polls: Vec<Vec<AssistantMessageEvent>>) -> Self {
        *self
            .deferred_poll_events
            .lock()
            .expect("deferred replay poll mutex") = Some(polls);
        self
    }
}

#[path = "replay_events.rs"]
mod replay_events;
use replay_events::*;
impl ReplayProvider {
    /// Reset the turn counter so a fresh run replays the recorded trace again.
    pub fn reset_turns(&self) {
        self.calls.store(0, Ordering::Release);
    }
}

fn validate_deferred_fetch(
    provider: &ReplayProvider,
    handle: &DeferredHandle,
    default_events: Option<&Vec<AssistantMessageEvent>>,
    has_polls: bool,
) -> Result<(), StreamError> {
    if default_events.is_none() && !has_polls {
        return Err(StreamError::Invalid(
            "replay provider has no deferred response fixture".into(),
        ));
    }
    if handle.id.is_empty() {
        return Err(StreamError::Invalid(
            "deferred response handle id is required".into(),
        ));
    }
    if provider.deferred_cancelled.load(Ordering::Acquire) {
        return Err(StreamError::Api(format!(
            "deferred response was cancelled: {}",
            handle.id
        )));
    }
    if provider.deferred_handle.as_ref().is_some_and(|expected| {
        expected.provider != handle.provider
            || expected.model_id != handle.model_id
            || expected.api != handle.api
    }) {
        return Err(StreamError::Invalid(format!(
            "deferred response handle scope mismatch: {}",
            handle.id
        )));
    }
    Ok(())
}

fn next_deferred_events(
    polls: &mut Option<Vec<Vec<AssistantMessageEvent>>>,
    default_events: Option<&Vec<AssistantMessageEvent>>,
) -> Result<Vec<AssistantMessageEvent>, StreamError> {
    match polls.as_mut() {
        Some(polls) if polls.is_empty() => Err(StreamError::Invalid(
            "deferred poll sequence is exhausted".into(),
        )),
        Some(polls) => Ok(polls.remove(0)),
        None => default_events
            .cloned()
            .ok_or_else(|| StreamError::Invalid("deferred response fixture is missing".into())),
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
        handle: &DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let default_events = self.deferred_events.as_ref();
        let has_polls = self
            .deferred_poll_events
            .lock()
            .map_err(|_| StreamError::Invalid("deferred replay poll state poisoned".into()))?
            .as_ref()
            .is_some_and(|polls| !polls.is_empty());
        validate_deferred_fetch(self, handle, default_events, has_polls)?;
        let mut polls = self
            .deferred_poll_events
            .lock()
            .map_err(|_| StreamError::Invalid("deferred replay poll state poisoned".into()))?;
        let events = next_deferred_events(&mut polls, default_events)?;
        Ok(Box::pin(stream::iter(events)))
    }

    async fn cancel_deferred(
        &self,
        _model: &Model,
        handle: &DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<(), StreamError> {
        if handle.id.is_empty() {
            return Err(StreamError::Invalid(
                "deferred response handle id is required".into(),
            ));
        }
        if let Some(expected) = &self.deferred_handle {
            if expected.provider != handle.provider
                || expected.model_id != handle.model_id
                || expected.api != handle.api
            {
                return Err(StreamError::Invalid(format!(
                    "deferred response handle scope mismatch: {}",
                    handle.id
                )));
            }
        }
        self.deferred_cancelled.store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(test)]
#[path = "replay_tests.rs"]
mod tests;
