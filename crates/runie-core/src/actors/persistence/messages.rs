//! Messages and handle for `PersistenceActor`.

use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::trust::TrustDecision;

/// Messages accepted by `PersistenceActor`.
#[derive(Debug, Clone)]
pub enum PersistenceMsg {
    /// Set a trust decision for a project path.
    SetTrust {
        path: PathBuf,
        decision: TrustDecision,
    },
    /// Append one entry to the input history file.
    AppendHistory { entry: String },
}

/// Handle for sending commands to a `PersistenceActor`.
#[derive(Clone, Debug)]
pub struct PersistenceActorHandle {
    tx: mpsc::Sender<PersistenceMsg>,
}

impl PersistenceActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<PersistenceMsg>) -> Self {
        Self { tx }
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        let _ = self.tx.send(PersistenceMsg::SetTrust { path, decision }).await;
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        let _ = self.tx.send(PersistenceMsg::AppendHistory { entry }).await;
    }
}
