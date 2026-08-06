//! Actor-owned transcript projection.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};

use crate::widgets::{Scrollback, ScrollbackMsg};

enum Command {
    ApplyBatch(Vec<ScrollbackMsg>, oneshot::Sender<()>),
}

struct Owner(tokio::task::JoinHandle<()>);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Clone)]
pub struct ScrollbackActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<Scrollback>,
    _owner: Arc<Owner>,
}

impl ScrollbackActor {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(32);
        let (snapshot_tx, snapshot) = watch::channel(Scrollback::new());
        // OWNER: ScrollbackActor — retained by every public handle clone.
        let owner = Arc::new(Owner(tokio::spawn(async move {
            let mut state = Scrollback::new();
            while let Some(command) = rx.recv().await {
                let (messages, reply) = match command {
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

    pub async fn apply(&self, message: ScrollbackMsg) {
        self.apply_batch(vec![message]).await;
    }

    pub async fn apply_batch(&self, messages: Vec<ScrollbackMsg>) {
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

    pub fn snapshot(&self) -> Scrollback {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Scrollback> {
        self.snapshot.clone()
    }
}

impl Default for ScrollbackActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollbackActor;
    use crate::widgets::{Line, LineKind, ScrollbackMsg};

    #[tokio::test]
    async fn actor_publishes_acknowledged_feed_snapshot() {
        let actor = ScrollbackActor::new();
        let index = actor.snapshot().lines().len();
        actor
            .apply(ScrollbackMsg::Append(Line::new(
                LineKind::Assistant,
                "hello",
            )))
            .await;
        assert_eq!(actor.snapshot().lines()[index].text, "hello");
    }

    #[tokio::test]
    async fn actor_reduces_completed_turn_summary_atomically() {
        let actor = ScrollbackActor::new();
        actor
            .apply(ScrollbackMsg::AppendTurnSummary("Worked for 1.0s".into()))
            .await;
        let lines = actor.snapshot();
        assert_eq!(lines.lines().len(), 2);
        assert_eq!(lines.lines()[1].kind, LineKind::TurnSummary);
        assert_eq!(lines.lines()[1].text, "Worked for 1.0s");
    }
}
