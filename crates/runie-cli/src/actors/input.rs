use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use runie_tui::tui::state::Msg;

/// InputActor reads terminal input events via crossterm and sends them as Msg variants.
///
/// Runs in a spawn_blocking task because crossterm's event polling is synchronous.
/// Responds to CancellationToken for graceful shutdown.
pub struct InputActor {
    msg_tx: mpsc::Sender<Msg>,
    cancel: CancellationToken,
}

impl InputActor {
    /// Create a new InputActor.
    pub fn new(msg_tx: mpsc::Sender<Msg>, cancel: CancellationToken) -> Self {
        Self { msg_tx, cancel }
    }

    /// Run the input actor. Consumes self.
    /// Polls crossterm events in a blocking task and sends them as Msg to the channel.
    /// Returns when the cancellation token is triggered.
    pub async fn run(self) {
        info!(target: "runie", "[ACTOR:Input] InputActor starting");

        let child_cancel = self.cancel.child_token();
        let msg_tx = self.msg_tx;

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

    /// Blocking event loop that polls crossterm and sends messages.
    fn poll_events(cancel: CancellationToken, msg_tx: mpsc::Sender<Msg>) {
        while !cancel.is_cancelled() {
            // Poll with 1ms timeout for responsive input
            if crossterm::event::poll(Duration::from_millis(1)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    let msgs = match event {
                        crossterm::event::Event::Resize(w, h) => vec![Msg::Resize(w, h)],
                        crossterm::event::Event::Paste(text) => vec![Msg::Paste(text)],
                        crossterm::event::Event::Key(key) => vec![Msg::TextareaKey(key)],
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
