//! Actor-owned status projection.

use std::sync::Arc;

use runie_core::types::AgentEvent;
use tokio::sync::{mpsc, oneshot, watch};

use crate::event_renderer::status_messages_for_event;
use crate::widgets::{StatusBar, StatusMsg};

enum Command {
    Apply(StatusMsg, oneshot::Sender<()>),
    ApplyBatch(Vec<StatusMsg>, oneshot::Sender<()>),
}

struct Owner(tokio::task::JoinHandle<()>);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Handle to the single owner of the status projection.
#[derive(Clone)]
pub struct StatusActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<StatusBar>,
    _owner: Arc<Owner>,
}

impl StatusActor {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(16);
        let (snapshot_tx, snapshot) = watch::channel(StatusBar::new());
        // OWNER: StatusActor — retained in every cloned public handle.
        let owner = Arc::new(Owner(tokio::spawn(async move {
            let mut state = StatusBar::new();
            while let Some(command) = rx.recv().await {
                let (messages, reply) = match command {
                    Command::Apply(message, reply) => (vec![message], reply),
                    Command::ApplyBatch(messages, reply) => (messages, reply),
                };
                for message in messages {
                    state.apply(message);
                }
                let _ = snapshot_tx.send(state.clone());
                let _ = reply.send(());
            }
        })));
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub async fn apply(&self, message: StatusMsg) {
        let (reply, acknowledged) = oneshot::channel();
        if self.tx.send(Command::Apply(message, reply)).await.is_ok() {
            let _ = acknowledged.await;
        }
    }

    /// Apply all status-owned transitions represented by one core event.
    /// Unknown events are intentionally a no-op for this projection.
    pub async fn apply_event(&self, event: &AgentEvent) {
        let messages = status_messages_for_event(event);
        if messages.is_empty() {
            return;
        }
        let (reply, acknowledged) = oneshot::channel();
        if self
            .tx
            .send(Command::ApplyBatch(messages, reply))
            .await
            .is_ok()
        {
            let _ = acknowledged.await;
        }
    }

    pub fn snapshot(&self) -> StatusBar {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<StatusBar> {
        self.snapshot.clone()
    }
}

impl Default for StatusActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::StatusActor;
    use crate::widgets::{Status, StatusMsg};
    use runie_core::types::AgentEvent;

    #[tokio::test]
    async fn actor_publishes_acknowledged_reducer_snapshot() {
        let actor = StatusActor::new();
        actor.apply(StatusMsg::Set(Status::Thinking)).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
    }

    #[tokio::test]
    async fn actor_applies_status_owned_core_event() {
        let actor = StatusActor::new();
        let mut updates = actor.subscribe();
        actor.apply_event(&AgentEvent::TurnStart).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
        assert!(updates.has_changed().expect("actor is alive"));
        updates.borrow_and_update();
        assert!(!updates.has_changed().expect("actor is alive"));
    }

    #[tokio::test]
    async fn actor_projects_agent_start_as_thinking() {
        let actor = StatusActor::new();
        actor.apply_event(&AgentEvent::AgentStart).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
    }
}
