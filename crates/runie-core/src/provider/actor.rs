//! `ProviderActor` — owns the one in-flight stream per assistant turn.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;

use crate::telemetry::{
    validate_pi_ai_request_end_attributes, SpanError, SpanStatus, TelemetrySpan,
};
use crate::types::{AgentContext, Model, SimpleStreamOptions};

use super::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn, WebSocketAdapter};
use crate::task_owner::{mailbox_ack, spawn_actor_worker, TaskOwner};

/// Broadcast capacity for stream events. Sized to absorb a burst of
/// `message_update` events without dropping.
const STREAM_CAPACITY: usize = 1024;

struct ProviderCommandContext<'a> {
    stream_fn: &'a Arc<dyn StreamFn>,
    websocket: Option<&'a Arc<dyn WebSocketAdapter>>,
    active_telemetry_span: &'a mut Option<TelemetrySpan>,
    pumps: &'a mut JoinSet<()>,
}

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

#[path = "actor_worker.rs"]
mod actor_worker;
use actor_worker::*;
#[path = "actor_telemetry.rs"]
mod actor_telemetry;
use actor_telemetry::*;
#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
