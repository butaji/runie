//! Core-side IPC queue handler.
//!
//! Receives [`Submission`]s from the TUI and sends [`Event`]s back.

use runie_protocol::event::Event;
use runie_protocol::op::Submission;
use tokio::sync::mpsc;

/// Capacity for the bounded async queues.
const QUEUE_CAPACITY: usize = 1024;

/// Core-side queue handler: receives submissions and sends events.
pub struct CoreIpc {
    /// Submission Queue receiver (TUI → Core).
    pub submissions: mpsc::Receiver<Submission>,
    /// Event Queue sender (Core → TUI).
    pub events: mpsc::Sender<Event>,
}

/// Channel ends held by the TUI side.
pub struct TuiQueueEnds {
    /// Submission Queue sender (TUI → Core).
    pub submissions: mpsc::Sender<Submission>,
    /// Event Queue receiver (Core → TUI).
    pub events: mpsc::Receiver<Event>,
}

/// Create a new bounded SQ/EQ pair.
pub fn core_ipc_pair() -> (CoreIpc, TuiQueueEnds) {
    let (sub_tx, sub_rx) = mpsc::channel(QUEUE_CAPACITY);
    let (evt_tx, evt_rx) = mpsc::channel(QUEUE_CAPACITY);
    let core = CoreIpc {
        submissions: sub_rx,
        events: evt_tx,
    };
    let tui = TuiQueueEnds {
        submissions: sub_tx,
        events: evt_rx,
    };
    (core, tui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_protocol::op::{Op, SubmissionId};

    #[tokio::test]
    async fn submission_queue_delivers_to_core() {
        let (mut core, tui) = core_ipc_pair();
        let sub = Submission::new(SubmissionId::new(1), Op::Interrupt);
        tui.submissions.send(sub.clone()).await.unwrap();
        let received = core.submissions.recv().await.unwrap();
        assert_eq!(received, sub);
    }
}
