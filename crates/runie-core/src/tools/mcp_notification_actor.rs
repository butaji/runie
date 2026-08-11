use super::McpNotificationQueue;
use super::McpStreamEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpNotificationSnapshot {
    pub queue: McpNotificationQueue,
}

impl McpNotificationSnapshot {
    pub fn terminal_lines(&self) -> Vec<String> {
        self.queue.terminal_lines()
    }
}

enum Command {
    Push {
        value: Value,
        reply: oneshot::Sender<()>,
    },
    Pop {
        reply: oneshot::Sender<Option<Value>>,
    },
    Clear {
        reply: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct McpNotificationActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<McpNotificationSnapshot>,
    _owner: Arc<crate::task_owner::TaskOwner>,
}

impl McpNotificationActor {
    pub fn new(capacity: usize) -> Self {
        let initial = McpNotificationSnapshot {
            queue: McpNotificationQueue::new(capacity),
        };
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (tx, owner) = crate::task_owner::spawn_actor_worker!(
            32,
            move |mut rx: mpsc::Receiver<Command>| async move {
                let mut state = initial;
                while let Some(command) = rx.recv().await {
                    reduce_command(&mut state, command);
                    let _ = snapshot_tx.send(state.clone());
                }
            }
        );
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }
    pub fn snapshot(&self) -> McpNotificationSnapshot {
        self.snapshot.borrow().clone()
    }
    pub async fn push(&self, value: Value) {
        let (reply, result) = oneshot::channel();
        let _ = self.tx.send(Command::Push { value, reply }).await;
        let _ = result.await;
    }
    pub async fn pop(&self) -> Option<Value> {
        let (reply, result) = oneshot::channel();
        let _ = self.tx.send(Command::Pop { reply }).await;
        result.await.unwrap_or(None)
    }
    pub async fn clear(&self) {
        let (reply, result) = oneshot::channel();
        let _ = self.tx.send(Command::Clear { reply }).await;
        let _ = result.await;
    }

    /// Admit only server notifications; responses remain owned by the
    /// request/response caller and are never mixed into this queue.
    pub async fn ingest_stream_events<I>(&self, events: I)
    where
        I: IntoIterator<Item = McpStreamEvent>,
    {
        for event in events {
            if event.data.get("id").is_none() {
                self.push(event.data).await;
            }
        }
    }
}

fn reduce_command(state: &mut McpNotificationSnapshot, command: Command) {
    match command {
        Command::Push { value, reply } => {
            state.queue.push(value);
            let _ = reply.send(());
        }
        Command::Pop { reply } => {
            let _ = reply.send(state.queue.pop());
        }
        Command::Clear { reply } => {
            state.queue.clear();
            let _ = reply.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn actor_publishes_inspectable_and_clearable_state() {
        let actor = McpNotificationActor::new(2);
        actor
            .push(serde_json::json!({"method":"notifications/progress"}))
            .await;
        assert_eq!(actor.snapshot().queue.pending.len(), 1);
        assert_eq!(
            actor.pop().await.unwrap()["method"],
            "notifications/progress"
        );
        actor
            .push(serde_json::json!({"method":"notifications/loggingMessage"}))
            .await;
        actor.clear().await;
        assert!(actor.snapshot().queue.pending.is_empty());
    }

    #[tokio::test]
    async fn actor_ingests_only_notification_stream_events() {
        let actor = McpNotificationActor::new(2);
        actor
            .ingest_stream_events([
                McpStreamEvent {
                    event: None,
                    data: serde_json::json!({"id": 1, "result": {}}),
                },
                McpStreamEvent {
                    event: Some("message".into()),
                    data: serde_json::json!({"method": "notifications/progress"}),
                },
            ])
            .await;
        assert_eq!(actor.snapshot().queue.pending.len(), 1);
        assert_eq!(
            actor.snapshot().queue.pending[0]["method"],
            "notifications/progress"
        );
    }
}
