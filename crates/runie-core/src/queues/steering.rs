//! Steering message queue actor.

use std::sync::Arc;

use tokio::sync::{mpsc, watch, Notify};

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
    Push(Box<AgentMessage>, mpsc::Sender<String>),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear(mpsc::Sender<Vec<String>>),
    Len(mpsc::Sender<usize>),
}

#[derive(Clone)]
pub struct SteeringQueueActor {
    tx: mpsc::Sender<SteeringCommand>,
    notify: Arc<Notify>,
    snapshot: watch::Receiver<SteeringQueueSnapshot>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<SteeringQueueSnapshot>>,
    _worker: Arc<TaskOwner>,
}

impl SteeringQueueActor {
    pub fn new() -> Self {
        let (tx, snapshot, shared_snapshot, worker) = spawn_steering_runtime();

        Self {
            tx,
            notify: Arc::new(Notify::new()),
            snapshot,
            shared_snapshot,
            _worker: worker,
        }
    }

    pub async fn push(&self, msg: AgentMessage) -> Option<String> {
        let id = mailbox_call!(
            self.tx,
            |reply| { SteeringCommand::Push(Box::new(msg), reply) },
            String::new()
        );
        self.notify.notify_one();
        (!id.is_empty()).then_some(id)
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        mailbox_call!(self.tx, SteeringCommand::DrainOne, None)
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        mailbox_call!(self.tx, SteeringCommand::DrainAll, Vec::new())
    }

    pub async fn clear(&self) -> Vec<String> {
        mailbox_call!(self.tx, SteeringCommand::Clear, Vec::new())
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

    pub fn snapshot(&self) -> SteeringQueueSnapshot {
        self.snapshot.borrow().clone()
    }
    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<SteeringQueueSnapshot> {
        self.shared_snapshot.borrow().clone()
    }
}

fn spawn_steering_runtime() -> (
    mpsc::Sender<SteeringCommand>,
    watch::Receiver<SteeringQueueSnapshot>,
    watch::Receiver<crate::SharedSnapshot<SteeringQueueSnapshot>>,
    Arc<TaskOwner>,
) {
    let initial = SteeringQueueSnapshot::default();
    let (snapshot_tx, snapshot) = watch::channel(initial.clone());
    let (shared_tx, shared_snapshot) = watch::channel(crate::SharedSnapshot::new(initial));
    let (tx, worker) = spawn_actor_worker!(64, move |rx| async move {
        run_steering_worker(rx, snapshot_tx, shared_tx).await;
    });
    (tx, snapshot, shared_snapshot, worker)
}

impl Default for SteeringQueueActor {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_steering_worker(
    mut rx: mpsc::Receiver<SteeringCommand>,
    snapshot_tx: watch::Sender<SteeringQueueSnapshot>,
    shared_tx: watch::Sender<crate::SharedSnapshot<SteeringQueueSnapshot>>,
) {
    let mut queue: Vec<(String, AgentMessage)> = Vec::new();
    let mut next_id = 1_u64;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            SteeringCommand::Push(msg, reply) => {
                let id = next_steering_id(&mut next_id);
                queue.push((id.clone(), *msg));
                let _ = reply.send(id).await;
                publish(&snapshot_tx, &shared_tx, queue.len());
            }
            SteeringCommand::DrainOne(reply) => {
                let popped = queue.first().is_some().then(|| queue.remove(0).1);
                let _ = reply.send(popped).await;
                publish(&snapshot_tx, &shared_tx, queue.len());
            }
            SteeringCommand::DrainAll(reply) => {
                let drained = std::mem::take(&mut queue)
                    .into_iter()
                    .map(|(_, message)| message)
                    .collect();
                let _ = reply.send(drained).await;
                publish(&snapshot_tx, &shared_tx, queue.len());
            }
            SteeringCommand::Clear(reply) => {
                let ids = queue.iter().map(|(id, _)| id.clone()).collect();
                queue.clear();
                let _ = reply.send(ids).await;
                publish(&snapshot_tx, &shared_tx, queue.len());
            }
            SteeringCommand::Len(reply) => {
                let _ = reply.send(queue.len()).await;
            }
        }
    }
}

fn next_steering_id(next_id: &mut u64) -> String {
    let id = format!("steer-{next_id}");
    *next_id += 1;
    id
}

fn publish(
    tx: &watch::Sender<SteeringQueueSnapshot>,
    shared: &watch::Sender<crate::SharedSnapshot<SteeringQueueSnapshot>>,
    len: usize,
) {
    crate::publish_shared_snapshot(
        tx,
        shared,
        SteeringQueueSnapshot {
            len,
            is_empty: len == 0,
        },
    );
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
        assert_eq!(q.push(msg(1)).await.as_deref(), Some("steer-1"));
        assert_eq!(q.push(msg(2)).await.as_deref(), Some("steer-2"));
        assert_eq!(q.push(msg(3)).await.as_deref(), Some("steer-3"));
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

    #[tokio::test]
    async fn snapshot_tracks_push_and_drain_as_shared_data() {
        let q = SteeringQueueActor::new();
        q.push(msg(1)).await;
        assert_eq!(q.snapshot().len, 1);
        assert!(!q.shared_snapshot().get().is_empty);
        q.drain_one().await;
        assert_eq!(q.shared_snapshot().get().len, 0);
        assert_eq!(q.shared_snapshot().strong_count(), 2);
    }
}
