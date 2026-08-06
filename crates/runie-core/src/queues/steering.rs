//! Steering message queue actor.

use std::sync::Arc;

use tokio::sync::{mpsc, Notify};

use crate::task_owner::{mailbox_call, spawn_actor_worker, TaskOwner};
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
    Push(Box<AgentMessage>),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear,
    Len(mpsc::Sender<usize>),
}

#[derive(Clone)]
pub struct SteeringQueueActor {
    tx: mpsc::Sender<SteeringCommand>,
    notify: Arc<Notify>,
    _worker: Arc<TaskOwner>,
}

impl SteeringQueueActor {
    pub fn new() -> Self {
        let notify = Arc::new(Notify::new());

        // OWNER: SteeringQueueActor
        let (tx, worker) = spawn_actor_worker!(64, move |rx| async move {
            run_steering_worker(rx).await;
        });

        Self {
            tx,
            notify,
            _worker: worker,
        }
    }

    pub async fn push(&self, msg: AgentMessage) {
        let _ = self.tx.send(SteeringCommand::Push(Box::new(msg))).await;
        self.notify.notify_one();
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        mailbox_call!(self.tx, SteeringCommand::DrainOne, None)
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        mailbox_call!(self.tx, SteeringCommand::DrainAll, Vec::new())
    }

    pub async fn clear(&self) {
        let _ = self.tx.send(SteeringCommand::Clear).await;
    }

    pub async fn len(&self) -> usize {
        mailbox_call!(self.tx, SteeringCommand::Len, 0)
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
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
            SteeringCommand::Push(msg) => queue.push(*msg),
            SteeringCommand::DrainOne(reply) => {
                // `Vec::drain(..1)` panics when the queue is empty, so
                // explicitly pop when there's at least one item.
                let popped = if queue.is_empty() {
                    None
                } else {
                    Some(queue.remove(0))
                };
                let _ = reply.send(popped).await;
            }
            SteeringCommand::DrainAll(reply) => {
                let drained = std::mem::take(&mut queue);
                let _ = reply.send(drained).await;
            }
            SteeringCommand::Clear => queue.clear(),
            SteeringCommand::Len(reply) => {
                let _ = reply.send(queue.len()).await;
            }
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

    #[tokio::test]
    async fn length_and_empty_projection_follow_queue_owner() {
        let q = SteeringQueueActor::new();
        assert!(q.is_empty().await);
        q.push(msg(1)).await;
        assert_eq!(q.len().await, 1);
        assert!(!q.is_empty().await);
        q.clear().await;
        assert!(q.is_empty().await);
    }
}
