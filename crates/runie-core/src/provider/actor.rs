//! `ProviderActor` — owns the one in-flight stream per assistant turn.

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;

use crate::telemetry::{SpanStatus, TelemetrySpan};
use crate::types::{AgentContext, Model, SimpleStreamOptions};

use super::stream_fn::{AssistantMessageEventStream, StreamFn};
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
}

#[derive(Clone)]
pub struct ProviderActor {
    tx: mpsc::Sender<ProviderCommand>,
    _worker: Arc<TaskOwner>,
}

impl ProviderActor {
    pub fn new(stream_fn: Arc<dyn StreamFn>) -> Self {
        let sf = stream_fn.clone();

        // OWNER: ProviderActor
        let (tx, worker) = spawn_actor_worker!(8, move |rx| async move {
            run_provider_worker(rx, sf).await;
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
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "the provider actor keeps supersession, capability setup, and reply ordering in one reducer"
)]
async fn run_provider_worker(
    mut rx: mpsc::Receiver<ProviderCommand>,
    stream_fn: Arc<dyn StreamFn>,
) {
    let mut pumps = JoinSet::new();
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
                let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
                let telemetry_span = if let Some(telemetry) = options
                    .as_ref()
                    .as_ref()
                    .and_then(|options| options.telemetry.clone())
                {
                    telemetry
                        .start_span(None, "pi.provider.stream", Default::default())
                        .await
                } else {
                    None
                };
                match stream_fn.stream(&model, &context, *options).await {
                    Ok(stream) => {
                        // Subscribe before starting the pump. Otherwise a
                        // fast replay stream can publish Start/tool events
                        // before the caller receives its broadcast receiver.
                        let receiver = event_tx.subscribe();
                        let tx = event_tx.clone();
                        // ProviderActor owns every active pump through this
                        // JoinSet; dropping the worker aborts its children.
                        pumps.spawn(pump_stream(stream, tx, telemetry_span));
                        let _ = reply.send(receiver);
                    }
                    Err(error) => {
                        if let Some(span) = telemetry_span {
                            span.status(SpanStatus::Error).await;
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
                let _ = reply.send(());
            }
        }
    }
}

async fn pump_stream(
    mut stream: AssistantMessageEventStream,
    tx: broadcast::Sender<crate::types::AssistantMessageEvent>,
    telemetry_span: Option<TelemetrySpan>,
) {
    use futures::StreamExt;
    while let Some(event) = stream.next().await {
        // Errors from the broadcast are non-fatal (no current receivers).
        let _ = tx.send(event);
        if let Some(span) = &telemetry_span {
            span.event("assistant.event").await;
        }
    }
    if let Some(span) = telemetry_span {
        span.status(SpanStatus::Ok).await;
        span.end().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::stream_fn::StreamError;
    use crate::types::{
        AgentContext, AssistantMessage, AssistantMessageEvent, Model, SimpleStreamOptions,
        StopReason, Usage,
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
        let mut rx = actor
            .start(Model::default(), AgentContext::default(), Some(options))
            .await
            .unwrap();
        while rx.recv().await.is_ok() {}
        let snapshot = telemetry.snapshot();
        assert_eq!(snapshot.spans.len(), 1);
        assert_eq!(snapshot.spans[0].events.len(), 3);
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
}
