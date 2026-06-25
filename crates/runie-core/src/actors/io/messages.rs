//! Messages and handle for `IoActor`.

use std::path::PathBuf;
use tokio::sync::mpsc;

/// Messages accepted by `IoActor`.
#[derive(Debug, Clone)]
pub enum IoMsg {
    /// Run a bash command and publish the output.
    RunBash { command: String },
    /// Write multiple files and publish the result.
    WriteFiles { edits: Vec<(PathBuf, String)> },
}

/// Handle for sending commands to an `IoActor`.
#[derive(Clone, Debug)]
pub struct IoActorHandle {
    tx: mpsc::Sender<IoMsg>,
}

impl IoActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<IoMsg>) -> Self {
        Self { tx }
    }

    /// Access the underlying sender (for routing intents from the DSL).
    pub fn tx(&self) -> &mpsc::Sender<IoMsg> {
        &self.tx
    }

    /// Request running a bash command.
    pub async fn run_bash(&self, command: String) {
        let _ = self.tx.send(IoMsg::RunBash { command }).await;
    }

    /// Request writing files.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        let _ = self.tx.send(IoMsg::WriteFiles { edits }).await;
    }
}
