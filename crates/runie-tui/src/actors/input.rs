//! InputActor — reads terminal input events via crossterm.
//!
//! Runs in a spawn_blocking task because crossterm's event polling is synchronous.

use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::Actor;
use crate::pipe::InputMsg;

/// InputActor for reading terminal input events.
pub struct InputActor;

impl InputActor {

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Blocking event loop that polls crossterm and sends messages.
    fn poll_events(cancel: CancellationToken, msg_tx: mpsc::Sender<InputMsg>) {
        while !cancel.is_cancelled() {
            // Poll with 50ms timeout — balances responsiveness (20fps) with CPU savings
            if crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    let msgs = match event {
                        crossterm::event::Event::Resize(w, h) => vec![InputMsg::Resize(w, h)],
                        crossterm::event::Event::Paste(text) => vec![InputMsg::Paste(text)],
                        crossterm::event::Event::Key(key) => vec![InputMsg::Key(key)],
                        _ => vec![],
                    };

                    for msg in msgs {
                        // Try once, if channel full drop the event (next poll will catch up)
                        let _ = msg_tx.try_send(msg);
                    }
                }
            }
        }
    }
}

impl Actor for InputActor {
    type Msg = InputMsg;

    fn name(&self) -> &'static str {
        "input"
    }

    async fn run(self, msg_tx: mpsc::Sender<InputMsg>, cancel: CancellationToken) {
        info!(target: "runie", "[ACTOR:Input] InputActor starting");

        let child_cancel = cancel.child_token();

        // Spawn blocking task for synchronous crossterm polling
        let handle = tokio::task::spawn_blocking(move || {
            Self::poll_events(child_cancel, msg_tx);
        });

        // Wait for the blocking task to complete
        // It completes when cancelled or on panic
        if let Err(e) = handle.await {
            tracing::error!(target: "runie", "[ACTOR:Input] InputActor task error: {}", e);
        }

        info!(target: "runie", "[ACTOR:Input] InputActor stopped");
    }
}

impl Default for InputActor {
    fn default() -> Self {
        Self::new()
    }
}
