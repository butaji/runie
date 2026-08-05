//! Steering message queue actor.

use std::sync::Arc;

use tokio::sync::{mpsc, Notify};

use crate::types::AgentMessage;

/// Snapshot of the steering queue for read-only consumers.
#[derive(Debug, Clone, Default)]
pub struct SteeringQueueSnapshot {
    pub len: usize,
    pub is_empty: bool,
}

/// Mailbox command for the steering queue actor.
#[derive(Debug)]
enum SteeringCommand {
    Push(AgentMessage),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear,
}

#[derive(Clone)]
pub struct SteeringQueueActor {
    tx: mpsc::Sender<SteeringCommand>,
    notify: Arc<Notify>,
}

impl SteeringQueueActor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        let notify = Arc::new(Notify::new());

        // OWNER: SteeringQueueActor
        tokio::spawn(async move {
            run_steering_worker(rx).await;
        });

        Self { tx, notify }
    }

    pub async fn push(&self, msg: AgentMessage) {
        let _ = self.tx.send(SteeringCommand::Push(msg)).await;
        self.notify.notify_one();
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        let _ = self.tx.send(SteeringCommand::DrainOne(reply_tx)).await;
        reply_rx.recv().await.flatten()
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        let _ = self.tx.send(SteeringCommand::DrainAll(reply_tx)).await;
        reply_rx.recv().await.unwrap_or_default()
    }

    pub async fn clear(&self) {
        let _ = self.tx.send(SteeringCommand::Clear).await;
    }

    pub fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }
}

impl Default for SteeringQueueActor {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_steering_worker(mut rx: mpsc::Receiver<SteeringCommand>) {
    let mut queue: Vec<AgentMessage> = Vec::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            SteeringCommand::Push(msg) => queue.push(msg),
            SteeringCommand::DrainOne(reply) => {
                // `Vec::drain(..1)` panics when the queue is empty, so
                // explicitly pop when there's at least one item.
                let popped = if queue.is_empty() { None } else { Some(queue.remove(0)) };
                let _ = reply.send(popped).await;
            }
            SteeringCommand::DrainAll(reply) => {
                let drained = std::mem::take(&mut queue);
                let _ = reply.send(drained).await;
            }
            SteeringCommand::Clear => queue.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{UserContent, UserMessage};

    fn msg(t: i64) -> AgentMessage {
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: format!("m{t}"),
            }],
            timestamp: t,
        })
    }

    #[tokio::test]
    async fn push_drain_one_drain_all() {
        let q = SteeringQueueActor::new();
        q.push(msg(1)).await;
        q.push(msg(2)).await;
        q.push(msg(3)).await;
        assert_eq!(q.drain_one().await.unwrap().timestamp(), 1);
        let rest = q.drain_all().await;
        assert_eq!(rest.len(), 2);
        assert_eq!(rest[0].timestamp(), 2);
        assert_eq!(rest[1].timestamp(), 3);
    }

    #[tokio::test]
    async fn clear_empties_queue() {
        let q = SteeringQueueActor::new();
        q.push(msg(1)).await;
        q.clear().await;
        assert!(q.drain_all().await.is_empty());
    }
}
