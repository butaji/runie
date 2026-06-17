//! IPC queue pair shared between core and TUI.
//!
//! Both endpoints live here so the core crate can expose a single constructor
//! without creating a circular dependency on the TUI crate.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use runie_protocol::{Event, Op, Submission, SubmissionId};

const CHANNEL_BUFFER: usize = 64;

/// Core side of the queue pair.
///
/// Receives [`Submission`]s from the TUI and sends [`Event`]s back.
#[derive(Debug)]
pub struct CoreIpc {
    sub_rx: mpsc::Receiver<Submission>,
    event_tx: mpsc::Sender<Event>,
}

impl CoreIpc {
    /// Create a connected `(CoreIpc, TuiIpc)` pair.
    pub fn new() -> (CoreIpc, TuiIpc) {
        let (sub_tx, sub_rx) = mpsc::channel::<Submission>(CHANNEL_BUFFER);
        let (event_tx, event_rx) = mpsc::channel::<Event>(CHANNEL_BUFFER);
        (
            CoreIpc { sub_rx, event_tx },
            TuiIpc {
                sub_tx,
                event_rx,
                next_id: AtomicU64::new(1),
            },
        )
    }

    /// Wait for the next submission from the TUI.
    pub async fn next_submission(&mut self) -> Option<Submission> {
        self.sub_rx.recv().await
    }

    /// Send an event to the TUI.
    pub async fn send_event(
        &self,
        event: Event,
    ) -> Result<(), mpsc::error::SendError<Event>> {
        self.event_tx.send(event).await
    }
}

/// TUI side of the queue pair.
///
/// Sends [`Submission`]s to the core and receives [`Event`]s back.
#[derive(Debug)]
pub struct TuiIpc {
    sub_tx: mpsc::Sender<Submission>,
    event_rx: mpsc::Receiver<Event>,
    next_id: AtomicU64,
}

impl TuiIpc {
    /// Create a connected `(CoreIpc, TuiIpc)` pair.
    pub fn new() -> (CoreIpc, TuiIpc) {
        CoreIpc::new()
    }

    /// Submit an operation to the core, returning its assigned id.
    pub async fn submit(&self, op: Op) -> Result<SubmissionId, mpsc::error::SendError<Submission>> {
        let id = SubmissionId(self.next_id.fetch_add(1, Ordering::SeqCst));
        let submission = Submission {
            id,
            op,
            trace: None,
        };
        self.sub_tx.send(submission).await?;
        Ok(id)
    }

    /// Wait for the next event from the core.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_protocol::{EventMsg, PromptOrigin};

    #[tokio::test]
    async fn submission_queue_delivers_to_core() {
        let (mut core, tui) = CoreIpc::new();
        let id = tui.submit(Op::Interrupt).await.unwrap();
        let sub = core.next_submission().await.expect("submission delivered");
        assert_eq!(sub.id, id);
        assert!(matches!(sub.op, Op::Interrupt));
    }

    #[tokio::test]
    async fn event_queue_delivers_to_tui() {
        let (core, mut tui) = CoreIpc::new();
        let event = Event {
            id: None,
            msg: EventMsg::TurnStarted { turn_id: 1 },
        };
        core.send_event(event.clone()).await.unwrap();
        let got = tui.next_event().await.expect("event delivered");
        assert_eq!(got, event);
    }

    #[tokio::test]
    async fn tui_submission_assigns_unique_ids() {
        let (_core, tui) = CoreIpc::new();
        let id_a = tui
            .submit(Op::UserTurn {
                input: "a".into(),
                origin: PromptOrigin::UserInput,
            })
            .await
            .unwrap();
        let id_b = tui
            .submit(Op::UserTurn {
                input: "b".into(),
                origin: PromptOrigin::UserInput,
            })
            .await
            .unwrap();
        assert_ne!(id_a, id_b);
    }
}
