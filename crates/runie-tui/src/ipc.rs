//! TUI-side IPC queue handler.
//!
//! Sends [`Submission`]s to the core and receives [`Event`]s back.

use runie_core::ipc::core_ipc_pair;
use runie_protocol::event::Event;
use runie_protocol::op::Submission;
use tokio::sync::mpsc;

/// TUI-side queue handler: sends submissions and receives events.
pub struct TuiIpc {
    /// Submission Queue sender (TUI → Core).
    pub submissions: mpsc::Sender<Submission>,
    /// Event Queue receiver (Core → TUI).
    pub events: mpsc::Receiver<Event>,
}

impl TuiIpc {
    /// Create a new TUI-side handler from channel ends.
    pub fn new(submissions: mpsc::Sender<Submission>, events: mpsc::Receiver<Event>) -> Self {
        Self {
            submissions,
            events,
        }
    }

    /// Send a submission to the core.
    pub async fn send_submission(
        &self,
        submission: Submission,
    ) -> Result<(), mpsc::error::SendError<Submission>> {
        self.submissions.send(submission).await
    }

    /// Receive the next event from the core.
    pub async fn recv_event(&mut self) -> Option<Event> {
        self.events.recv().await
    }
}

/// Create a new SQ/EQ pair wired for the TUI.
pub fn tui_ipc_pair() -> (runie_core::ipc::CoreIpc, TuiIpc) {
    let (core, tui_ends) = core_ipc_pair();
    let tui = TuiIpc::new(tui_ends.submissions, tui_ends.events);
    (core, tui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_protocol::event::EventMsg;
    use runie_protocol::op::{Op, SubmissionId};

    #[tokio::test]
    async fn event_queue_delivers_to_tui() {
        let (core, mut tui) = tui_ipc_pair();
        let event = Event::correlated(SubmissionId::new(7), EventMsg::TurnStarted { turn_id: 3 });
        core.events.send(event.clone()).await.unwrap();
        let received = tui.recv_event().await.unwrap();
        assert_eq!(received, event);
    }

    #[tokio::test]
    async fn submission_queue_delivers_to_core_via_tui() {
        let (mut core, tui) = tui_ipc_pair();
        let sub = Submission::new(SubmissionId::new(2), Op::Shutdown);
        tui.send_submission(sub.clone()).await.unwrap();
        let received = core.submissions.recv().await.unwrap();
        assert_eq!(received, sub);
    }
}
