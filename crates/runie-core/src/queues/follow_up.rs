//! Follow-up message queue actor. Mirror of steering.

use std::sync::Arc;

use tokio::sync::{mpsc, watch, Notify};

use crate::task_owner::{mailbox_call, spawn_actor_worker, TaskOwner};
use crate::types::AgentMessage;

#[derive(Debug, Clone, Default)]
pub struct FollowUpQueueSnapshot {
    pub len: usize,
    pub is_empty: bool,
}

#[derive(Debug)]
enum FollowUpCommand {
    Push(Box<AgentMessage>, mpsc::Sender<String>),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear(mpsc::Sender<Vec<String>>),
    Len(mpsc::Sender<usize>),
}

#[derive(Clone)]
pub struct FollowUpQueueActor {
    tx: mpsc::Sender<FollowUpCommand>,
    notify: Arc<Notify>,
    snapshot: watch::Receiver<FollowUpQueueSnapshot>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<FollowUpQueueSnapshot>>,
    _worker: Arc<TaskOwner>,
}

impl FollowUpQueueActor {
    pub fn new() -> Self {
        let (tx, snapshot, shared_snapshot, worker) = spawn_follow_up_runtime();
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
            |reply| { FollowUpCommand::Push(Box::new(msg), reply) },
            String::new()
        );
        self.notify.notify_one();
        (!id.is_empty()).then_some(id)
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        mailbox_call!(self.tx, FollowUpCommand::DrainOne, None)
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        mailbox_call!(self.tx, FollowUpCommand::DrainAll, Vec::new())
    }

    pub async fn clear(&self) -> Vec<String> {
        mailbox_call!(self.tx, FollowUpCommand::Clear, Vec::new())
    }

    pub async fn len(&self) -> usize {
        mailbox_call!(self.tx, FollowUpCommand::Len, 0)
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    pub fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub fn snapshot(&self) -> FollowUpQueueSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<FollowUpQueueSnapshot> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(
        &self,
    ) -> watch::Receiver<crate::SharedSnapshot<FollowUpQueueSnapshot>> {
        self.shared_snapshot.clone()
    }
}

fn spawn_follow_up_runtime() -> (
    mpsc::Sender<FollowUpCommand>,
    watch::Receiver<FollowUpQueueSnapshot>,
    watch::Receiver<crate::SharedSnapshot<FollowUpQueueSnapshot>>,
    Arc<TaskOwner>,
) {
    let initial = FollowUpQueueSnapshot::default();
    let (snapshot_tx, snapshot) = watch::channel(initial.clone());
    let (shared_tx, shared_snapshot) = watch::channel(crate::SharedSnapshot::new(initial));
    let (tx, worker) = spawn_actor_worker!(64, move |rx| async move {
        run_follow_up_worker(rx, snapshot_tx, shared_tx).await;
    });
    (tx, snapshot, shared_snapshot, worker)
}

impl Default for FollowUpQueueActor {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_follow_up_worker(
    mut rx: mpsc::Receiver<FollowUpCommand>,
    snapshot_tx: watch::Sender<FollowUpQueueSnapshot>,
    shared_tx: watch::Sender<crate::SharedSnapshot<FollowUpQueueSnapshot>>,
) {
    let mut queue: Vec<(String, AgentMessage)> = Vec::new();
    let mut next_id = 1_u64;

    while let Some(cmd) = rx.recv().await {
        apply_follow_up_command(cmd, &mut queue, &mut next_id, &snapshot_tx, &shared_tx).await;
    }
}

async fn apply_follow_up_command(
    cmd: FollowUpCommand,
    queue: &mut Vec<(String, AgentMessage)>,
    next_id: &mut u64,
    snapshot_tx: &watch::Sender<FollowUpQueueSnapshot>,
    shared_tx: &watch::Sender<crate::SharedSnapshot<FollowUpQueueSnapshot>>,
) {
    match cmd {
        FollowUpCommand::Push(msg, reply) => {
            let id = format!("follow-up-{next_id}");
            *next_id += 1;
            queue.push((id.clone(), *msg));
            let _ = reply.send(id).await;
            publish(snapshot_tx, shared_tx, queue.len());
        }
        FollowUpCommand::DrainOne(reply) => {
            let popped = queue.first().is_some().then(|| queue.remove(0).1);
            let _ = reply.send(popped).await;
            publish(snapshot_tx, shared_tx, queue.len());
        }
        FollowUpCommand::DrainAll(reply) => {
            let drained = std::mem::take(queue)
                .into_iter()
                .map(|(_, message)| message)
                .collect();
            let _ = reply.send(drained).await;
            publish(snapshot_tx, shared_tx, queue.len());
        }
        FollowUpCommand::Clear(reply) => {
            let ids = queue.iter().map(|(id, _)| id.clone()).collect();
            queue.clear();
            let _ = reply.send(ids).await;
            publish(snapshot_tx, shared_tx, queue.len());
        }
        FollowUpCommand::Len(reply) => {
            let _ = reply.send(queue.len()).await;
        }
    }
}

fn publish(
    snapshot_tx: &watch::Sender<FollowUpQueueSnapshot>,
    shared_tx: &watch::Sender<crate::SharedSnapshot<FollowUpQueueSnapshot>>,
    len: usize,
) {
    crate::publish_shared_snapshot(
        snapshot_tx,
        shared_tx,
        FollowUpQueueSnapshot {
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
    async fn push_drain_all_in_order() {
        let q = FollowUpQueueActor::new();
        assert_eq!(q.push(msg(1)).await.as_deref(), Some("follow-up-1"));
        assert_eq!(q.push(msg(2)).await.as_deref(), Some("follow-up-2"));
        let all = q.drain_all().await;
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].timestamp(), 1);
        assert_eq!(all[1].timestamp(), 2);
    }

    #[tokio::test]
    async fn length_and_empty_projection_follow_queue_owner() {
        let q = FollowUpQueueActor::new();
        assert!(q.is_empty().await);
        q.push(msg(1)).await;
        assert_eq!(q.len().await, 1);
        assert!(!q.is_empty().await);
        q.clear().await;
        assert!(q.is_empty().await);
    }

    #[tokio::test]
    async fn shared_snapshot_tracks_push_and_drain_data() {
        let q = FollowUpQueueActor::new();
        q.push(msg(1)).await;
        assert_eq!(q.snapshot().len, 1);
        assert!(!q.shared_snapshot().get().is_empty);
        q.drain_one().await;
        assert_eq!(q.shared_snapshot().get().len, 0);
        assert_eq!(q.shared_snapshot().strong_count(), 2);
        assert_eq!(q.shared_subscribe().borrow().get().len, 0);
    }
}
