//! Follow-up message queue actor. Mirror of steering.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Notify};

use crate::task_owner::{mailbox_ack, mailbox_call, spawn_actor_worker, TaskOwner};
use crate::types::AgentMessage;

#[derive(Debug, Clone, Default)]
pub struct FollowUpQueueSnapshot {
    pub len: usize,
    pub is_empty: bool,
}

#[derive(Debug)]
enum FollowUpCommand {
    Push(Box<AgentMessage>, oneshot::Sender<()>),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear(oneshot::Sender<()>),
    Len(mpsc::Sender<usize>),
}

#[derive(Clone)]
pub struct FollowUpQueueActor {
    tx: mpsc::Sender<FollowUpCommand>,
    notify: Arc<Notify>,
    _worker: Arc<TaskOwner>,
}

impl FollowUpQueueActor {
    pub fn new() -> Self {
        let notify = Arc::new(Notify::new());

        // OWNER: FollowUpQueueActor
        let (tx, worker) = spawn_actor_worker!(64, move |rx| async move {
            run_follow_up_worker(rx).await;
        });

        Self {
            tx,
            notify,
            _worker: worker,
        }
    }

    pub async fn push(&self, msg: AgentMessage) {
        let _ = mailbox_ack!(self.tx, |reply| {
            FollowUpCommand::Push(Box::new(msg), reply)
        });
        self.notify.notify_one();
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        mailbox_call!(self.tx, FollowUpCommand::DrainOne, None)
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        mailbox_call!(self.tx, FollowUpCommand::DrainAll, Vec::new())
    }

    pub async fn clear(&self) {
        let _ = mailbox_ack!(self.tx, FollowUpCommand::Clear);
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
}

impl Default for FollowUpQueueActor {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_follow_up_worker(mut rx: mpsc::Receiver<FollowUpCommand>) {
    let mut queue: Vec<AgentMessage> = Vec::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            FollowUpCommand::Push(msg, reply) => {
                queue.push(*msg);
                let _ = reply.send(());
            }
            FollowUpCommand::DrainOne(reply) => {
                // `Vec::drain(..1)` panics on an empty queue; pop instead.
                let popped = if queue.is_empty() {
                    None
                } else {
                    Some(queue.remove(0))
                };
                let _ = reply.send(popped).await;
            }
            FollowUpCommand::DrainAll(reply) => {
                let drained = std::mem::take(&mut queue);
                let _ = reply.send(drained).await;
            }
            FollowUpCommand::Clear(reply) => {
                queue.clear();
                let _ = reply.send(());
            }
            FollowUpCommand::Len(reply) => {
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
    async fn push_drain_all_in_order() {
        let q = FollowUpQueueActor::new();
        q.push(msg(1)).await;
        q.push(msg(2)).await;
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
}
