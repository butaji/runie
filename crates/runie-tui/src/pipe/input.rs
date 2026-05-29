//! Input handling pipe.
//!
//! Shell for Phase 3 — reads terminal input events and converts to InputMsg.
//!
//! Note: Full implementation with tokio will be added in Phase 3.
//! For now this is a placeholder that compiles.

use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum InputMsg {
    Key(KeyEvent),
    Paste(String),
    Resize(u16, u16),
}

pub struct InputPipe {
    // TODO: add tokio channels in Phase 3
    // msg_tx: mpsc::Sender<InputMsg>,
    // cancel: CancellationToken,
}

impl InputPipe {
    pub fn new() -> Self {
        Self {}
    }

    /// Start the input pipe (placeholder for Phase 3).
    /// Will be async with tokio in Phase 3.
    pub async fn run(self) {
        // TODO: implement with tokio in Phase 3
        // For now, input handling is done via the existing event::poll loop
        // in the Tui struct
        tracing::debug!("[InputPipe] placeholder run() called");
    }
}

impl Default for InputPipe {
    fn default() -> Self {
        Self::new()
    }
}