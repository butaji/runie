use super::*;
use crate::provider::stream_fn::StreamError;
use crate::types::{
    AgentContext, AssistantMessage, AssistantMessageEvent, DeferredHandle, Model,
    SimpleStreamOptions, StopReason, Usage,
};
use futures::stream;

struct ThreeEventFn;
#[async_trait::async_trait]
impl StreamFn for ThreeEventFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let events = vec![
            AssistantMessageEvent::Start {
                partial: crate::types::AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            },
        ];
        let s = stream::iter(events);
        Ok(Box::pin(s))
    }
}

struct WebsocketFn;
#[async_trait::async_trait]
impl WebSocketAdapter for WebsocketFn {
    async fn stream_websocket(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }])))
    }
}

struct PendingFn;
#[async_trait::async_trait]
impl StreamFn for PendingFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Ok(Box::pin(futures::stream::pending()))
    }
}

struct ErrorFn;
#[async_trait::async_trait]
impl StreamFn for ErrorFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Err(StreamError::Api("bad request".into()))
    }
}

struct ErrorEventFn;
#[async_trait::async_trait]
impl StreamFn for ErrorEventFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Error {
            reason: StopReason::Error,
            error: AssistantMessage::with_error(StopReason::Error, "stream failed"),
        }])))
    }
}

struct SummarizerFn;
#[async_trait::async_trait]
impl StreamFn for SummarizerFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Err(StreamError::Invalid("stream unused".into()))
    }

    async fn summarize_compaction(
        &self,
        request: &crate::session::CompactionSummaryRequest,
    ) -> Result<crate::session::CompactionSummary, StreamError> {
        Ok(crate::session::CompactionSummary {
            summary: format!("{} tokens", request.tokens_before),
            usage: None,
            details: Some(serde_json::json!({"adapter": "test"})),
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, StreamError> {
        Ok(vec![Model {
            provider: "test".into(),
            id: "catalog-model".into(),
            name: "Catalog Model".into(),
            ..Model::default()
        }])
    }
}

struct DeferredFn {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

#[async_trait::async_trait]
impl StreamFn for DeferredFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Err(StreamError::Invalid("ordinary stream unused".into()))
    }

    async fn fetch_deferred(
        &self,
        _model: &Model,
        handle: &DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        assert_eq!(handle.id, "deferred-1");
        Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }])))
    }

    async fn cancel_deferred(
        &self,
        _model: &Model,
        handle: &DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<(), StreamError> {
        assert_eq!(handle.id, "deferred-1");
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}

#[tokio::test]
async fn websocket_transport_uses_only_the_injected_provider_adapter() {
    let provider =
        ProviderActor::new_with_websocket(Arc::new(ErrorFn), Some(Arc::new(WebsocketFn)));
    let mut events = provider
        .start(
            Model::default(),
            AgentContext::default(),
            Some(SimpleStreamOptions {
                transport: Some(crate::types::ProviderTransport::Websocket),
                ..Default::default()
            }),
        )
        .await
        .expect("provider reply");
    assert!(matches!(
        events.recv().await.expect("websocket event"),
        AssistantMessageEvent::Done { .. }
    ));
}

#[tokio::test]
async fn forward_three_events() {
    let actor = ProviderActor::new(std::sync::Arc::new(ThreeEventFn));
    let mut rx = actor
        .start(
            Model {
                id: "test".into(),
                name: "test".into(),
                api: "test".into(),
                provider: "test".into(),
                base_url: String::new(),
                reasoning: false,
                context_window: 0,
                max_tokens: 0,
                ..Default::default()
            },
            AgentContext::default(),
            None,
        )
        .await
        .unwrap();
    let mut count = 0;
    while rx.recv().await.is_ok() {
        count += 1;
        if count == 3 {
            break;
        }
    }
    assert_eq!(count, 3);
}

#[tokio::test]
async fn provider_stream_projects_telemetry_through_owned_capability() {
    let telemetry = crate::telemetry::TelemetryActor::new();
    let actor = ProviderActor::new(std::sync::Arc::new(ThreeEventFn));
    let options = SimpleStreamOptions {
        telemetry: Some(telemetry.clone()),
        ..Default::default()
    };
    let model = Model {
        provider: "test-provider".into(),
        id: "test-model".into(),
        api: "test-api".into(),
        ..Model::default()
    };
    let mut rx = actor
        .start(model, AgentContext::default(), Some(options))
        .await
        .unwrap();
    while rx.recv().await.is_ok() {}
    let snapshot = telemetry.snapshot();
    assert_stream_telemetry(&snapshot);
}

fn assert_stream_telemetry(snapshot: &crate::telemetry::TelemetrySnapshot) {
    assert_stream_span_shape(snapshot);
    assert_stream_usage_shape(snapshot);
    assert_stream_timing_shape(snapshot);
}

fn assert_stream_span_shape(snapshot: &crate::telemetry::TelemetrySnapshot) {
    assert_eq!(snapshot.spans.len(), 1);
    assert!(snapshot.spans[0].events.is_empty());
    assert_eq!(snapshot.spans[0].name, "pi.ai.request");
    assert_eq!(snapshot.spans[0].attributes["pi.ai.operation"], "stream");
    assert_eq!(
        snapshot.spans[0].attributes["pi.ai.response.stop_reason"],
        "stop"
    );
}

fn assert_stream_usage_shape(snapshot: &crate::telemetry::TelemetrySnapshot) {
    assert_eq!(snapshot.spans[0].attributes["pi.ai.usage.total_tokens"], 0);
    assert_eq!(
        snapshot.spans[0].attributes["pi.ai.usage.cache_read_tokens"],
        0
    );
    assert_eq!(
        snapshot.spans[0].attributes["pi.ai.usage.cache_write_tokens"],
        0
    );
    assert_eq!(
        snapshot.spans[0].attributes["pi.ai.usage.reasoning_tokens"],
        0
    );
    assert_eq!(snapshot.spans[0].attributes["pi.ai.usage.cost"], 0.0);
}

fn assert_stream_timing_shape(snapshot: &crate::telemetry::TelemetrySnapshot) {
    assert_eq!(snapshot.spans[0].attributes["pi.ai.stream.chunk_count"], 1);
    assert!(
        snapshot.spans[0].attributes["pi.ai.stream.time_to_first_chunk_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
    assert!(snapshot.spans[0].ended);
}

#[path = "actor_tests_extra.rs"]
mod extra;
