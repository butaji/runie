//! Actor-owned session journal projection.
//!
//! This is the first persistence seam for Pi-compatible session behavior:
//! message entries are appended from the typed event bus, never by the TUI or
//! provider adapters. Storage backends can consume the immutable snapshot
//! later without becoming a second state owner.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};

use crate::events::EventBus;
use crate::task_owner::{spawn_actor_worker, spawn_owned_worker, TaskOwner};
use crate::types::{AgentEvent, AgentMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub id: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub timestamp: i64,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub sequence: u64,
    pub leaf_id: Option<String>,
    pub entries: Vec<SessionEntry>,
}

enum Command {
    Append(Box<AgentMessage>, oneshot::Sender<()>),
    Reset(oneshot::Sender<()>),
    Flush(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct SessionActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<SessionSnapshot>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl SessionActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(SessionSnapshot::default());
        let (tx, owner) = spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Command>| async move {
            let mut state = SessionSnapshot::default();
            let mut next_id = 1_u64;
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Append(message, reply) => {
                        state.sequence += 1;
                        let id = format!("entry-{}", next_id);
                        next_id += 1;
                        let entry = SessionEntry {
                            id: id.clone(),
                            seq: state.sequence,
                            parent_id: state.leaf_id.clone(),
                            timestamp: message.timestamp(),
                            message: *message,
                        };
                        state.leaf_id = Some(id);
                        state.entries.push(entry);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Reset(reply) => {
                        state = SessionSnapshot::default();
                        next_id = 1;
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Flush(reply) => {
                        let _ = reply.send(());
                    }
                }
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

    pub fn new_with_bus(bus: &EventBus) -> Self {
        let mut actor = Self::new();
        let events = bus.subscribe();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            let mut events = events;
            while let Ok(event) = events.recv().await {
                match event {
                    AgentEvent::MessageEnd { message } => {
                        let (reply, done) = oneshot::channel();
                        if tx
                            .send(Command::Append(Box::new(message), reply))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        let _ = done.await;
                    }
                    AgentEvent::Reset => {
                        let (reply, done) = oneshot::channel();
                        if tx.send(Command::Reset(reply)).await.is_err() {
                            break;
                        }
                        let _ = done.await;
                    }
                    _ => {}
                }
            }
        }));
        actor
    }

    pub async fn append(&self, message: AgentMessage) {
        let (reply, done) = oneshot::channel();
        if self
            .tx
            .send(Command::Append(Box::new(message), reply))
            .await
            .is_ok()
        {
            let _ = done.await;
        }
    }

    pub async fn reset(&self) {
        let (reply, done) = oneshot::channel();
        if self.tx.send(Command::Reset(reply)).await.is_ok() {
            let _ = done.await;
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot.borrow().clone()
    }

    pub async fn flush(&self) {
        let (reply, done) = oneshot::channel();
        if self.tx.send(Command::Flush(reply)).await.is_ok() {
            let _ = done.await;
        }
    }
}

impl Default for SessionActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use crate::types::{UserContent, UserMessage};

    fn user(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: text.into() }],
            timestamp: 7,
        })
    }

    #[tokio::test]
    async fn actor_reduces_ordered_entries_and_parent_links() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.sequence, 2);
        assert_eq!(snapshot.entries[0].parent_id, None);
        assert_eq!(snapshot.entries[1].parent_id.as_deref(), Some("entry-1"));
        assert_eq!(snapshot.leaf_id.as_deref(), Some("entry-2"));
    }

    #[tokio::test]
    async fn bus_message_end_and_reset_are_the_only_projection_inputs() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::MessageEnd {
            message: user("one"),
        });
        tokio::task::yield_now().await;
        assert_eq!(actor.snapshot().entries.len(), 1);
        bus.publish(AgentEvent::Reset);
        tokio::task::yield_now().await;
        assert!(actor.snapshot().entries.is_empty());
    }
}
