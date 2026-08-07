//! Actor-owned telemetry capability for Pi-compatible span lifecycles.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use crate::task_owner::{spawn_actor_worker, TaskOwner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSnapshot {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<TelemetryEventSnapshot>,
    pub status: SpanStatus,
    pub ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryEventSnapshot {
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub next_id: u64,
    pub spans: Vec<SpanSnapshot>,
}

/// Runtime-editable telemetry replay commands. These address the actor
/// mailbox contract and never expose reducer state to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryAction {
    Start {
        #[serde(default)]
        parent_id: Option<u64>,
        name: String,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },
    Event {
        id: u64,
        name: String,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },
    SetAttributes {
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
    },
    Status {
        id: u64,
        status: SpanStatus,
    },
    End {
        id: u64,
    },
}

/// A complete actor replay case, loaded from YAML without recompilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryScenario {
    pub actions: Vec<TelemetryAction>,
    pub expected: TelemetrySnapshot,
}

enum TelemetryCommand {
    Start {
        parent_id: Option<u64>,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<u64>,
    },
    Event {
        id: u64,
        name: String,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    },
    SetAttributes {
        id: u64,
        attributes: HashMap<String, serde_json::Value>,
        reply: oneshot::Sender<()>,
    },
    Status {
        id: u64,
        status: SpanStatus,
        reply: oneshot::Sender<()>,
    },
    End {
        id: u64,
        reply: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct TelemetryActor {
    tx: mpsc::Sender<TelemetryCommand>,
    snapshot: watch::Receiver<TelemetrySnapshot>,
    _owner: Arc<TaskOwner>,
}

impl TelemetryActor {
    #[allow(
        clippy::too_many_lines,
        reason = "the actor reducer keeps the complete span lifecycle in one explicit boundary"
    )]
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(TelemetrySnapshot::default());
        let (tx, owner) = spawn_actor_worker!(32, move |mut rx: mpsc::Receiver<
            TelemetryCommand,
        >| async move {
            let mut state = TelemetrySnapshot::default();
            while let Some(command) = rx.recv().await {
                match command {
                    TelemetryCommand::Start {
                        parent_id,
                        name,
                        attributes,
                        reply,
                    } => {
                        let id = state.next_id;
                        state.next_id = state.next_id.wrapping_add(1);
                        state.spans.push(SpanSnapshot {
                            id,
                            parent_id,
                            name,
                            attributes,
                            events: Vec::new(),
                            status: SpanStatus::Unset,
                            ended: false,
                        });
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(id);
                    }
                    TelemetryCommand::Event {
                        id,
                        name,
                        attributes,
                        reply,
                    } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.events
                                .push(TelemetryEventSnapshot { name, attributes });
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::SetAttributes {
                        id,
                        attributes,
                        reply,
                    } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.attributes.extend(attributes);
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::Status { id, status, reply } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.status = status;
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                    TelemetryCommand::End { id, reply } => {
                        if let Some(span) = state
                            .spans
                            .iter_mut()
                            .find(|span| span.id == id && !span.ended)
                        {
                            span.ended = true;
                            let _ = snapshot_tx.send(state.clone());
                        }
                        let _ = reply.send(());
                    }
                }
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        self.snapshot.borrow().clone()
    }

    /// Apply one declarative replay action through the actor mailbox.
    #[allow(
        clippy::too_many_lines,
        reason = "the replay DSL maps each declared actor command explicitly"
    )]
    pub async fn apply(&self, action: TelemetryAction) -> Option<u64> {
        match action {
            TelemetryAction::Start {
                parent_id,
                name,
                attributes,
            } => self
                .start_span(parent_id, name, attributes)
                .await
                .map(|span| span.id),
            TelemetryAction::Event {
                id,
                name,
                attributes,
            } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                }
                .event(name, attributes)
                .await;
                None
            }
            TelemetryAction::SetAttributes { id, attributes } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                }
                .set_attributes(attributes)
                .await;
                None
            }
            TelemetryAction::Status { id, status } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                }
                .status(status)
                .await;
                None
            }
            TelemetryAction::End { id } => {
                TelemetrySpan {
                    actor: self.clone(),
                    id,
                }
                .end()
                .await;
                None
            }
        }
    }

    pub async fn replay(&self, actions: impl IntoIterator<Item = TelemetryAction>) {
        for action in actions {
            let _ = self.apply(action).await;
        }
    }

    pub async fn start_span(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<TelemetrySpan> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(TelemetryCommand::Start {
                parent_id,
                name: name.into(),
                attributes,
                reply,
            })
            .await
            .ok()?;
        Some(TelemetrySpan {
            actor: self.clone(),
            id: result.await.ok()?,
        })
    }

    /// Execute a callback inside an actor-owned span, matching Pi's
    /// callback-scoped `startSpan` contract. Completion always settles the
    /// span through mailbox commands before the result is returned.
    pub async fn with_span<F, Fut, T, E>(
        &self,
        parent_id: Option<u64>,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
        callback: F,
    ) -> Option<Result<T, E>>
    where
        F: FnOnce(TelemetrySpan) -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let span = self.start_span(parent_id, name, attributes).await?;
        let result = callback(span.clone()).await;
        match &result {
            Ok(_) => span.status(SpanStatus::Ok).await,
            Err(_) => span.status(SpanStatus::Error).await,
        }
        span.end().await;
        Some(result)
    }
}

impl Default for TelemetryActor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TelemetrySpan {
    actor: TelemetryActor,
    pub id: u64,
}

impl TelemetrySpan {
    pub async fn event(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) {
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Event {
                id: self.id,
                name: name.into(),
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn set_attributes(&self, attributes: HashMap<String, serde_json::Value>) {
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::SetAttributes {
                id: self.id,
                attributes,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn status(&self, status: SpanStatus) {
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::Status {
                id: self.id,
                status,
                reply,
            })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn end(&self) {
        let (reply, acknowledged) = oneshot::channel();
        let _ = self
            .actor
            .tx
            .send(TelemetryCommand::End { id: self.id, reply })
            .await;
        let _ = acknowledged.await;
    }

    pub async fn child(
        &self,
        name: impl Into<String>,
        attributes: HashMap<String, serde_json::Value>,
    ) -> Option<Self> {
        self.actor.start_span(Some(self.id), name, attributes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nested_spans_and_terminal_state_are_actor_owned() {
        let actor = TelemetryActor::new();
        let root = actor.start_span(None, "run", HashMap::new()).await.unwrap();
        let child = root.child("request", HashMap::new()).await.unwrap();
        child.event("headers", HashMap::new()).await;
        child.status(SpanStatus::Ok).await;
        child.end().await;
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.spans[1].parent_id, Some(root.id));
        assert_eq!(snapshot.spans[1].events[0].name, "headers");
        assert_eq!(snapshot.spans[1].status, SpanStatus::Ok);
        assert!(snapshot.spans[1].ended);
    }

    #[tokio::test]
    async fn ended_spans_ignore_late_mutations() {
        let actor = TelemetryActor::new();
        let span = actor.start_span(None, "run", HashMap::new()).await.unwrap();
        span.end().await;
        span.event("late", HashMap::new()).await;
        assert!(actor.snapshot().spans[0].events.is_empty());
    }

    #[tokio::test]
    async fn callback_scoped_span_settles_success_and_error_through_actor() {
        let actor = TelemetryActor::new();
        let success = actor
            .with_span(None, "success", HashMap::new(), |span| async move {
                span.event("finished", HashMap::new()).await;
                Ok::<_, &'static str>("done")
            })
            .await
            .unwrap()
            .unwrap();
        let failure = actor
            .with_span(None, "failure", HashMap::new(), |_span| async {
                Err::<(), _>("failed")
            })
            .await
            .unwrap();
        assert_eq!(success, "done");
        assert_eq!(failure, Err("failed"));
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.spans[0].status, SpanStatus::Ok);
        assert_eq!(snapshot.spans[1].status, SpanStatus::Error);
        assert!(snapshot.spans.iter().all(|span| span.ended));
    }
}
