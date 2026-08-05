//! `ProviderActor` — owns the one in-flight stream per assistant turn.

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};

use crate::types::{AgentContext, Model, SimpleStreamOptions};

use super::stream_fn::{AssistantMessageEventStream, StreamFn};

/// Broadcast capacity for stream events. Sized to absorb a burst of
/// `message_update` events without dropping.
const STREAM_CAPACITY: usize = 1024;

pub enum ProviderCommand {
    Start {
        model: Model,
        context: AgentContext,
        options: Option<SimpleStreamOptions>,
        reply: oneshot::Sender<broadcast::Receiver<crate::types::AssistantMessageEvent>>,
    },
    Cancel,
}

#[derive(Clone)]
pub struct ProviderActor {
    tx: mpsc::Sender<ProviderCommand>,
    stream_fn: Arc<dyn StreamFn>,
}

impl ProviderActor {
    pub fn new(stream_fn: Arc<dyn StreamFn>) -> Self {
        let (tx, rx) = mpsc::channel(8);
        let sf = stream_fn.clone();

        // OWNER: ProviderActor
        tokio::spawn(async move {
            run_provider_worker(rx, sf).await;
        });

        Self { tx, stream_fn }
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
                model,
                context,
                options,
                reply: reply_tx,
            })
            .await;
        reply_rx.await.ok()
    }

    pub async fn cancel(&self) {
        let _ = self.tx.send(ProviderCommand::Cancel).await;
    }
}

async fn run_provider_worker(
    mut rx: mpsc::Receiver<ProviderCommand>,
    stream_fn: Arc<dyn StreamFn>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            ProviderCommand::Start {
                model,
                context,
                options,
                reply,
            } => {
                let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
                match stream_fn.stream(&model, &context, options).await {
                    Ok(stream) => {
                        // Subscribe before starting the pump. Otherwise a
                        // fast replay stream can publish Start/tool events
                        // before the caller receives its broadcast receiver.
                        let receiver = event_tx.subscribe();
                        let tx = event_tx.clone();
                        // OWNER: ProviderActor — wraps the stream in an owned task.
                        tokio::spawn(pump_stream(stream, tx));
                        let _ = reply.send(receiver);
                    }
                    Err(_) => {
                        let _ = reply.send(event_tx.subscribe());
                    }
                }
            }
            ProviderCommand::Cancel => {
                // Cancellation is currently best-effort; the next Start will
                // simply run after the in-flight stream finishes (or errors).
            }
        }
    }
}

async fn pump_stream(
    mut stream: AssistantMessageEventStream,
    tx: broadcast::Sender<crate::types::AssistantMessageEvent>,
) {
    use futures::StreamExt;
    while let Some(event) = stream.next().await {
        // Errors from the broadcast are non-fatal (no current receivers).
        let _ = tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::stream_fn::StreamError;
    use crate::types::{
        AgentContext, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, Usage,
    };
    use futures::stream;
    use std::sync::Arc;

    struct ThreeEventFn;
    #[async_trait::async_trait]
    impl StreamFn for ThreeEventFn {
        async fn stream(
            &self,
            _model: &Model,
            _context: &crate::types::AgentContext,
            _options: Option<SimpleStreamOptions>,
        ) -> Result<AssistantMessageEventStream, StreamError> {
            use futures::stream::Stream;
            let events = vec![
                AssistantMessageEvent::Start,
                AssistantMessageEvent::TextDelta { delta: "hi".into() },
                AssistantMessageEvent::Done {
                    stop_reason: StopReason::Stop,
                    usage: Usage::default(),
                },
            ];
            let s = stream::iter(events);
            Ok(Box::pin(s))
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
}
