//! `ProviderActor` — owns the one in-flight stream per assistant turn.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;

use crate::telemetry::{SpanError, SpanStatus, TelemetrySpan};
use crate::types::{AgentContext, Model, SimpleStreamOptions};

use super::stream_fn::{AssistantMessageEventStream, StreamFn, WebSocketAdapter};
use crate::task_owner::{mailbox_ack, spawn_actor_worker, TaskOwner};

/// Broadcast capacity for stream events. Sized to absorb a burst of
/// `message_update` events without dropping.
const STREAM_CAPACITY: usize = 1024;

pub enum ProviderCommand {
    Start {
        model: Box<Model>,
        context: Box<AgentContext>,
        options: Box<Option<SimpleStreamOptions>>,
        reply: oneshot::Sender<broadcast::Receiver<crate::types::AssistantMessageEvent>>,
    },
    Cancel {
        reply: oneshot::Sender<()>,
    },
    FetchDeferred {
        model: Box<Model>,
        handle: crate::types::DeferredHandle,
        options: Box<Option<SimpleStreamOptions>>,
        reply: oneshot::Sender<
            Result<
                broadcast::Receiver<crate::types::AssistantMessageEvent>,
                crate::provider::stream_fn::StreamError,
            >,
        >,
    },
    CancelDeferred {
        model: Box<Model>,
        handle: crate::types::DeferredHandle,
        options: Box<Option<SimpleStreamOptions>>,
        reply: oneshot::Sender<Result<(), crate::provider::stream_fn::StreamError>>,
    },
    SummarizeCompaction {
        request: Box<crate::session::CompactionSummaryRequest>,
        reply: oneshot::Sender<
            Result<crate::session::CompactionSummary, crate::provider::stream_fn::StreamError>,
        >,
    },
    ListModels {
        reply: oneshot::Sender<Result<Vec<Model>, crate::provider::stream_fn::StreamError>>,
    },
}

#[derive(Clone)]
pub struct ProviderActor {
    tx: mpsc::Sender<ProviderCommand>,
    _worker: Arc<TaskOwner>,
}

impl ProviderActor {
    pub fn new(stream_fn: Arc<dyn StreamFn>) -> Self {
        Self::new_with_websocket(stream_fn, None)
    }

    pub fn new_with_websocket(
        stream_fn: Arc<dyn StreamFn>,
        websocket: Option<Arc<dyn WebSocketAdapter>>,
    ) -> Self {
        let sf = stream_fn.clone();

        // OWNER: ProviderActor
        let (tx, worker) = spawn_actor_worker!(8, move |rx| async move {
            run_provider_worker(rx, sf, websocket).await;
        });

        Self {
            tx,
            _worker: worker,
        }
    }

    pub async fn start(
        &self,
        model: Model,
        context: AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Option<broadcast::Receiver<crate::types::AssistantMessageEvent>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderCommand::Start {
                model: Box::new(model),
                context: Box::new(context),
                options: Box::new(options),
                reply: reply_tx,
            })
            .await;
        reply_rx.await.ok()
    }

    pub async fn cancel(&self) {
        let _ = mailbox_ack!(self.tx, |reply| ProviderCommand::Cancel { reply });
    }

    pub async fn fetch_deferred(
        &self,
        model: Model,
        handle: crate::types::DeferredHandle,
        options: Option<SimpleStreamOptions>,
    ) -> Result<
        broadcast::Receiver<crate::types::AssistantMessageEvent>,
        crate::provider::stream_fn::StreamError,
    > {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderCommand::FetchDeferred {
                model: Box::new(model),
                handle,
                options: Box::new(options),
                reply: reply_tx,
            })
            .await;
        reply_rx.await.unwrap_or_else(|_| {
            Err(crate::provider::stream_fn::StreamError::Invalid(
                "provider actor stopped".into(),
            ))
        })
    }

    pub async fn cancel_deferred(
        &self,
        model: Model,
        handle: crate::types::DeferredHandle,
        options: Option<SimpleStreamOptions>,
    ) -> Result<(), crate::provider::stream_fn::StreamError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderCommand::CancelDeferred {
                model: Box::new(model),
                handle,
                options: Box::new(options),
                reply: reply_tx,
            })
            .await;
        reply_rx.await.unwrap_or_else(|_| {
            Err(crate::provider::stream_fn::StreamError::Invalid(
                "provider actor stopped".into(),
            ))
        })
    }

    pub async fn summarize_compaction(
        &self,
        request: crate::session::CompactionSummaryRequest,
    ) -> Result<crate::session::CompactionSummary, crate::provider::stream_fn::StreamError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderCommand::SummarizeCompaction {
                request: Box::new(request),
                reply: reply_tx,
            })
            .await;
        reply_rx.await.unwrap_or_else(|_| {
            Err(crate::provider::stream_fn::StreamError::Invalid(
                "provider actor stopped".into(),
            ))
        })
    }

    pub async fn list_models(&self) -> Result<Vec<Model>, crate::provider::stream_fn::StreamError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderCommand::ListModels { reply: reply_tx })
            .await;
        reply_rx.await.unwrap_or_else(|_| {
            Err(crate::provider::stream_fn::StreamError::Invalid(
                "provider actor stopped".into(),
            ))
        })
    }
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "the provider actor keeps supersession, capability setup, and reply ordering in one reducer"
)]
async fn run_provider_worker(
    mut rx: mpsc::Receiver<ProviderCommand>,
    stream_fn: Arc<dyn StreamFn>,
    websocket: Option<Arc<dyn WebSocketAdapter>>,
) {
    let mut pumps = JoinSet::new();
    let mut active_telemetry_span = None;
    while let Some(cmd) = rx.recv().await {
        while pumps.try_join_next().is_some() {}
        match cmd {
            ProviderCommand::Start {
                model,
                context,
                options,
                reply,
            } => {
                // Pi owns one active provider request per turn. A new start
                // supersedes any still-running pump before its receiver is
                // handed back, so two streams cannot publish concurrently.
                pumps.abort_all();
                settle_aborted_telemetry_span(&mut active_telemetry_span).await;
                let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
                let telemetry_span = if let Some(telemetry) = options
                    .as_ref()
                    .as_ref()
                    .and_then(|options| options.telemetry.clone())
                {
                    telemetry
                        .start_span(
                            None,
                            "pi.ai.request",
                            HashMap::from([
                                ("pi.ai.operation".into(), serde_json::json!("stream")),
                                (
                                    "pi.ai.provider".into(),
                                    serde_json::json!(model.provider.clone()),
                                ),
                                ("pi.ai.model".into(), serde_json::json!(model.id.clone())),
                                ("pi.ai.api".into(), serde_json::json!(model.api.clone())),
                                ("pi.ai.streaming".into(), serde_json::json!(true)),
                            ]),
                        )
                        .await
                } else {
                    None
                };
                let stream_result = match options
                    .as_ref()
                    .as_ref()
                    .and_then(|options| options.transport)
                {
                    Some(crate::types::ProviderTransport::Websocket)
                    | Some(crate::types::ProviderTransport::WebsocketCached) => {
                        match websocket.as_ref() {
                            Some(adapter) => adapter
                                .stream_websocket(&model, &context, *options)
                                .await,
                            None => Err(crate::provider::stream_fn::StreamError::Invalid(
                                "websocket transport requires a provider-specific websocket adapter".into(),
                            )),
                        }
                    }
                    _ => stream_fn.stream(&model, &context, *options).await,
                };
                match stream_result {
                    Ok(stream) => {
                        // Subscribe before starting the pump. Otherwise a
                        // fast replay stream can publish Start/tool events
                        // before the caller receives its broadcast receiver.
                        let receiver = event_tx.subscribe();
                        let tx = event_tx.clone();
                        // ProviderActor owns every active pump through this
                        // JoinSet; dropping the worker aborts its children.
                        active_telemetry_span = telemetry_span.clone();
                        pumps.spawn(pump_stream(stream, tx, telemetry_span));
                        let _ = reply.send(receiver);
                    }
                    Err(error) => {
                        if let Some(span) = telemetry_span {
                            span.status_with_error(
                                SpanStatus::Error,
                                Some(SpanError {
                                    name: "ProviderError".into(),
                                    message: error.to_string(),
                                }),
                            )
                            .await;
                            span.end().await;
                        }
                        let receiver = event_tx.subscribe();
                        let _ = event_tx.send(crate::types::AssistantMessageEvent::Error {
                            reason: crate::types::StopReason::Error,
                            error: crate::types::AssistantMessage::with_error(
                                crate::types::StopReason::Error,
                                error.to_string(),
                            ),
                        });
                        let _ = reply.send(receiver);
                    }
                }
            }
            ProviderCommand::Cancel { reply } => {
                // pi aborts the active provider request. The actor owns every
                // pump in this JoinSet, so aborting the set cancels the
                // in-flight stream without detaching a task.
                pumps.abort_all();
                settle_aborted_telemetry_span(&mut active_telemetry_span).await;
                let _ = reply.send(());
            }
            ProviderCommand::FetchDeferred {
                model,
                handle,
                options,
                reply,
            } => {
                let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
                match stream_fn.fetch_deferred(&model, &handle, *options).await {
                    Ok(stream) => {
                        let receiver = event_tx.subscribe();
                        pumps.spawn(pump_stream(stream, event_tx, None));
                        let _ = reply.send(Ok(receiver));
                    }
                    Err(error) => {
                        let _ = reply.send(Err(error));
                    }
                }
            }
            ProviderCommand::CancelDeferred {
                model,
                handle,
                options,
                reply,
            } => {
                let result = stream_fn.cancel_deferred(&model, &handle, *options).await;
                let _ = reply.send(result);
            }
            ProviderCommand::SummarizeCompaction { request, reply } => {
                let result = stream_fn.summarize_compaction(&request).await;
                let _ = reply.send(result);
            }
            ProviderCommand::ListModels { reply } => {
                let result = stream_fn.list_models().await;
                let _ = reply.send(result);
            }
        }
    }
}

async fn settle_aborted_telemetry_span(span: &mut Option<TelemetrySpan>) {
    if let Some(span) = span.take() {
        span.status_with_error(
            SpanStatus::Error,
            Some(SpanError {
                name: "AbortError".into(),
                message: "provider stream aborted".into(),
            }),
        )
        .await;
        span.end().await;
    }
}

#[allow(
    clippy::cognitive_complexity,
    reason = "the owned stream pump keeps event delivery and telemetry settlement ordered"
)]
async fn pump_stream(
    mut stream: AssistantMessageEventStream,
    tx: broadcast::Sender<crate::types::AssistantMessageEvent>,
    telemetry_span: Option<TelemetrySpan>,
) {
    use futures::StreamExt;
    while let Some(event) = stream.next().await {
        if let Some(span) = &telemetry_span {
            let attributes = telemetry_attributes_for_event(&event);
            if !attributes.is_empty() {
                span.set_attributes(attributes).await;
            }
        }
        // Errors from the broadcast are non-fatal (no current receivers).
        let _ = tx.send(event);
    }
    if let Some(span) = telemetry_span {
        span.status(SpanStatus::Ok).await;
        span.end().await;
    }
}

fn telemetry_attributes_for_event(
    event: &crate::types::AssistantMessageEvent,
) -> HashMap<String, serde_json::Value> {
    match event {
        crate::types::AssistantMessageEvent::Done {
            stop_reason, usage, ..
        } => HashMap::from([
            (
                "pi.ai.response.stop_reason".into(),
                serde_json::json!(telemetry_stop_reason(*stop_reason)),
            ),
            (
                "pi.ai.usage.input_tokens".into(),
                serde_json::json!(usage.input),
            ),
            (
                "pi.ai.usage.output_tokens".into(),
                serde_json::json!(usage.output),
            ),
            (
                "pi.ai.usage.total_tokens".into(),
                serde_json::json!(usage.total_tokens),
            ),
        ]),
        crate::types::AssistantMessageEvent::Error { reason, .. } => HashMap::from([
            (
                "pi.ai.response.stop_reason".into(),
                serde_json::json!(telemetry_stop_reason(*reason)),
            ),
            ("pi.ai.error.type".into(), serde_json::json!("provider")),
        ]),
        _ => HashMap::new(),
    }
}

fn telemetry_stop_reason(reason: crate::types::StopReason) -> &'static str {
    match reason {
        crate::types::StopReason::Stop => "stop",
        crate::types::StopReason::ToolUse => "tool_use",
        crate::types::StopReason::MaxTokens => "length",
        crate::types::StopReason::Error => "error",
        crate::types::StopReason::Aborted => "aborted",
        crate::types::StopReason::Pending => "pending",
        crate::types::StopReason::Deferred => "deferred",
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(snapshot.spans.len(), 1);
        assert!(snapshot.spans[0].events.is_empty());
        assert_eq!(snapshot.spans[0].name, "pi.ai.request");
        assert_eq!(snapshot.spans[0].attributes["pi.ai.operation"], "stream");
        assert_eq!(
            snapshot.spans[0].attributes["pi.ai.response.stop_reason"],
            "stop"
        );
        assert_eq!(snapshot.spans[0].attributes["pi.ai.usage.total_tokens"], 0);
        assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
        assert!(snapshot.spans[0].ended);
    }

    #[tokio::test]
    async fn cancel_aborts_owned_stream_pump() {
        let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
        let mut rx = actor
            .start(Model::default(), AgentContext::default(), None)
            .await
            .unwrap();
        actor.cancel().await;
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
            .await
            .expect("cancel should close the stream");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cancel_settles_the_owned_provider_telemetry_span() {
        let telemetry = crate::telemetry::TelemetryActor::new();
        let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
        let options = SimpleStreamOptions {
            telemetry: Some(telemetry.clone()),
            ..Default::default()
        };
        let mut events = actor
            .start(Model::default(), AgentContext::default(), Some(options))
            .await
            .unwrap();
        actor.cancel().await;
        assert!(events.recv().await.is_err());
        let snapshot = telemetry.snapshot();
        assert_eq!(snapshot.spans.len(), 1);
        assert!(snapshot.spans[0].ended);
        assert_eq!(snapshot.spans[0].status, SpanStatus::Error);
        assert_eq!(snapshot.spans[0].error.as_ref().unwrap().name, "AbortError");
    }

    #[tokio::test]
    async fn a_new_start_supersedes_the_previous_owned_stream() {
        let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
        let mut previous = actor
            .start(Model::default(), AgentContext::default(), None)
            .await
            .unwrap();
        let _current = actor
            .start(Model::default(), AgentContext::default(), None)
            .await
            .unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), previous.recv())
            .await
            .expect("superseded stream should close");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn startup_error_is_encoded_as_assistant_error_event() {
        let actor = ProviderActor::new(std::sync::Arc::new(ErrorFn));
        let mut rx = actor
            .start(Model::default(), AgentContext::default(), None)
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await.unwrap(),
            crate::types::AssistantMessageEvent::Error { error, .. }
                if error.error_text() == "api: bad request"
        ));
    }

    #[tokio::test]
    async fn unsupported_deferred_commands_return_explicit_capability_errors() {
        let actor = ProviderActor::new(std::sync::Arc::new(ThreeEventFn));
        let handle = DeferredHandle {
            provider: "test".into(),
            model_id: "test".into(),
            api: "test".into(),
            id: "deferred-1".into(),
            expires_at: None,
            poll_after_ms: None,
            data: None,
        };

        let fetch = actor
            .fetch_deferred(Model::default(), handle.clone(), None)
            .await;
        assert!(matches!(
            fetch,
            Err(StreamError::Invalid(message))
                if message == "provider does not support deferred responses"
        ));

        let cancel = actor.cancel_deferred(Model::default(), handle, None).await;
        assert!(matches!(
            cancel,
            Err(StreamError::Invalid(message))
                if message == "provider cannot cancel deferred responses"
        ));
    }

    #[tokio::test]
    async fn deferred_fetch_and_cancel_use_the_injected_provider_capability() {
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let actor = ProviderActor::new(Arc::new(DeferredFn {
            cancelled: cancelled.clone(),
        }));
        let handle = DeferredHandle {
            provider: "test".into(),
            model_id: "test".into(),
            api: "test".into(),
            id: "deferred-1".into(),
            expires_at: None,
            poll_after_ms: None,
            data: Some(serde_json::json!({"continuation": true})),
        };

        let mut events = actor
            .fetch_deferred(Model::default(), handle.clone(), None)
            .await
            .expect("deferred fetch capability");
        assert!(matches!(
            events.recv().await.expect("deferred event"),
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                ..
            }
        ));

        actor
            .cancel_deferred(Model::default(), handle, None)
            .await
            .expect("deferred cancellation capability");
        assert!(cancelled.load(std::sync::atomic::Ordering::Acquire));
    }

    #[tokio::test]
    async fn compaction_summary_uses_owned_provider_capability() {
        let actor = ProviderActor::new(Arc::new(SummarizerFn));
        let summary = actor
            .summarize_compaction(crate::session::CompactionSummaryRequest {
                history: vec![],
                turn_prefix: vec![],
                retained_tail: vec![],
                tokens_before: 42,
                previous_summary: None,
                custom_instructions: None,
            })
            .await
            .expect("summary capability");
        assert_eq!(summary.summary, "42 tokens");
        assert_eq!(
            summary.details,
            Some(serde_json::json!({"adapter": "test"}))
        );
    }

    #[tokio::test]
    async fn model_discovery_uses_owned_provider_capability() {
        let actor = ProviderActor::new(Arc::new(SummarizerFn));
        let models = actor.list_models().await.expect("model discovery");
        assert_eq!(models[0].id, "catalog-model");
    }
}
