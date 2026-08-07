//! Actor-owned status projection.

use std::sync::Arc;

use runie_core::types::AgentEvent;
use runie_core::{
    mailbox_ack, mailbox_batch_ack, spawn_actor_worker, spawn_owned_worker, task_owner::TaskOwner,
};
use tokio::sync::{mpsc, oneshot, watch};

use crate::widgets::{Status, StatusBar, StatusMsg, StatusSnapshot};
use runie_tui_model::status_messages_for_event;

enum Command {
    Apply(StatusMsg, oneshot::Sender<()>),
    ApplyBatch(Vec<StatusMsg>, oneshot::Sender<()>),
}

/// Handle to the single owner of the status projection.
#[derive(Clone)]
pub struct StatusActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<StatusSnapshot>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl StatusActor {
    pub fn new() -> Self {
        let initial = StatusSnapshot {
            elapsed_ticks: crate::clock::parity_elapsed_ticks().unwrap_or_default(),
            ..StatusSnapshot::default()
        };
        let elapsed_seed = crate::clock::parity_elapsed_ticks();
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (tx, owner) = spawn_actor_worker!(16, |mut rx: mpsc::Receiver<Command>| async move {
            let mut state = initial;
            while let Some(command) = rx.recv().await {
                let (messages, reply) = match command {
                    Command::Apply(message, reply) => (vec![message], reply),
                    Command::ApplyBatch(messages, reply) => (messages, reply),
                };
                for message in messages {
                    state.apply(message, elapsed_seed);
                }
                let _ = snapshot_tx.send(state.clone());
                let _ = reply.send(());
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

    /// Construct a live status projection that owns its event-bus subscription.
    /// The renderer remains a pure event consumer and no longer mutates this
    /// actor as a side effect of drawing the feed.
    pub fn new_with_bus(bus: &runie_core::events::EventBus) -> Self {
        let mut actor = Self::new();
        let mut events = bus.subscribe();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        let messages = vec![StatusMsg::Set(Status::Error(format!(
                            "event stream lagged ({count} events)",
                        )))];
                        if !mailbox_batch_ack!(tx, messages, |messages, reply| {
                            Command::ApplyBatch(messages, reply)
                        }) {
                            break;
                        }
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let messages = status_messages_for_event(&event);
                if !mailbox_batch_ack!(tx, messages, |messages, reply| {
                    Command::ApplyBatch(messages, reply)
                }) {
                    break;
                }
            }
        }));
        actor
    }

    pub async fn apply(&self, message: StatusMsg) {
        let _ = mailbox_ack!(self.tx, |reply| Command::Apply(message, reply));
    }

    /// Apply all status-owned transitions represented by one core event.
    /// Unknown events are intentionally a no-op for this projection.
    pub async fn apply_event(&self, event: &AgentEvent) {
        let messages = status_messages_for_event(event);
        let _ = mailbox_batch_ack!(self.tx, messages, |messages, reply| {
            Command::ApplyBatch(messages, reply)
        });
    }

    pub fn snapshot(&self) -> StatusBar {
        StatusBar::from_model_snapshot(self.snapshot.borrow().clone())
    }

    pub fn model_snapshot(&self) -> StatusSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<StatusSnapshot> {
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
        let model = actor.model_snapshot();
        assert_eq!(model.state, Status::Thinking);
        assert_eq!(model.theme, runie_core::types::ThemeKind::GrokNight);
        assert!(model.turn_usage.is_none());
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

    #[tokio::test]
    async fn bus_owned_actor_reduces_status_events_without_renderer_dispatch() {
        let bus = runie_core::events::EventBus::new();
        let actor = StatusActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::AgentStart);
        snapshot.changed().await.expect("status bus projection");
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
    }
}
