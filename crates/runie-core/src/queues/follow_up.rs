//! Follow-up message queue actor. Mirror of steering.

use std::sync::Arc;

use tokio::sync::{mpsc, Notify};

use crate::types::AgentMessage;

#[derive(Debug, Clone, Default)]
pub struct FollowUpQueueSnapshot {
    pub len: usize,
    pub is_empty: bool,
}

#[derive(Debug)]
enum FollowUpCommand {
    Push(AgentMessage),
    DrainOne(mpsc::Sender<Option<AgentMessage>>),
    DrainAll(mpsc::Sender<Vec<AgentMessage>>),
    Clear,
}

#[derive(Clone)]
pub struct FollowUpQueueActor {
    tx: mpsc::Sender<FollowUpCommand>,
    notify: Arc<Notify>,
}

impl FollowUpQueueActor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        let notify = Arc::new(Notify::new());

        // OWNER: FollowUpQueueActor
        tokio::spawn(async move {
            run_follow_up_worker(rx).await;
        });

        Self { tx, notify }
    }

    pub async fn push(&self, msg: AgentMessage) {
        let _ = self.tx.send(FollowUpCommand::Push(msg)).await;
        self.notify.notify_one();
    }

    pub async fn drain_one(&self) -> Option<AgentMessage> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        let _ = self.tx.send(FollowUpCommand::DrainOne(reply_tx)).await;
        reply_rx.recv().await.flatten()
    }

    pub async fn drain_all(&self) -> Vec<AgentMessage> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        let _ = self.tx.send(FollowUpCommand::DrainAll(reply_tx)).await;
        reply_rx.recv().await.unwrap_or_default()
    }

    pub async fn clear(&self) {
        let _ = self.tx.send(FollowUpCommand::Clear).await;
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
            FollowUpCommand::Push(msg) => queue.push(msg),
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
            FollowUpCommand::Clear => queue.clear(),
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
}
