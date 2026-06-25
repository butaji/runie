//! Unified messages and handle for `SessionActor`.
//!
//! Combines responsibilities from the former `PersistenceActor`,
//! `SessionStoreActor`, and `session_actor.rs`.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::session::Session;
use crate::trust::TrustDecision;

// ============================================================================
// Trust + History (formerly PersistenceActor)
// ============================================================================

/// Messages accepted by `SessionActor` — trust and history.
#[derive(Debug, Clone)]
pub enum PersistenceMsg {
    /// Set a trust decision for a project path.
    SetTrust { path: PathBuf, decision: TrustDecision },
    /// Append one entry to the input history file.
    AppendHistory { entry: String },
}

// ============================================================================
// Session CRUD (formerly SessionStoreActor)
// ============================================================================

/// Messages accepted by `SessionActor` — session management.
#[derive(Debug, Clone)]
pub enum SessionStoreMsg {
    /// Load a named session and publish its durable events.
    Load { name: String },
    /// Save a session snapshot under the given name.
    Save { name: String, session: Session },
    /// Delete a named session.
    Delete { name: String },
    /// Import a session snapshot from a file path.
    Import { path: PathBuf },
    /// Export a session snapshot to a file path.
    Export { path: PathBuf, session: Session },
    /// List all saved sessions.
    List,
}

// ============================================================================
// Unified SessionActor
// ============================================================================

/// Unified message enum for the `SessionActor`.
#[derive(Debug, Clone)]
pub enum SessionMsg {
    // Trust + history
    SetTrust { path: PathBuf, decision: TrustDecision },
    AppendHistory { entry: String },
    // Session CRUD
    Load { name: String },
    Save { name: String, session: Session },
    Delete { name: String },
    Import { path: PathBuf },
    Export { path: PathBuf, session: Session },
    List,
}

impl From<PersistenceMsg> for SessionMsg {
    fn from(m: PersistenceMsg) -> Self {
        match m {
            PersistenceMsg::SetTrust { path, decision } => SessionMsg::SetTrust { path, decision },
            PersistenceMsg::AppendHistory { entry } => SessionMsg::AppendHistory { entry },
        }
    }
}

impl From<SessionStoreMsg> for SessionMsg {
    fn from(m: SessionStoreMsg) -> Self {
        match m {
            SessionStoreMsg::Load { name } => SessionMsg::Load { name },
            SessionStoreMsg::Save { name, session } => SessionMsg::Save { name, session },
            SessionStoreMsg::Delete { name } => SessionMsg::Delete { name },
            SessionStoreMsg::Import { path } => SessionMsg::Import { path },
            SessionStoreMsg::Export { path, session } => SessionMsg::Export { path, session },
            SessionStoreMsg::List => SessionMsg::List,
        }
    }
}

/// Handle for sending commands to a `SessionActor`.
#[derive(Clone, Debug)]
pub struct SessionActorHandle {
    tx: mpsc::Sender<SessionMsg>,
}

impl SessionActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { tx }
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        let _ = self.tx.send(SessionMsg::SetTrust { path, decision }).await;
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        let _ = self.tx.send(SessionMsg::AppendHistory { entry }).await;
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Load { name }).await;
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: Session) {
        let _ = self.tx.send(SessionMsg::Save { name, session }).await;
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Delete { name }).await;
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: PathBuf) {
        let _ = self.tx.send(SessionMsg::Import { path }).await;
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: PathBuf, session: Session) {
        let _ = self.tx.send(SessionMsg::Export { path, session }).await;
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        let _ = self.tx.send(SessionMsg::List).await;
    }
}

impl From<SessionActorHandle> for PersistenceActorHandle {
    fn from(h: SessionActorHandle) -> Self {
        PersistenceActorHandle { tx: h.tx }
    }
}

impl From<SessionActorHandle> for SessionStoreActorHandle {
    fn from(h: SessionActorHandle) -> Self {
        SessionStoreActorHandle { tx: h.tx }
    }
}

/// Backward-compatible handle for trust/history operations.
#[derive(Clone, Debug)]
pub struct PersistenceActorHandle {
    pub(crate) tx: mpsc::Sender<SessionMsg>,
}

impl PersistenceActorHandle {
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { tx }
    }

    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        let _ = self.tx.send(SessionMsg::SetTrust { path, decision }).await;
    }

    pub async fn append_history(&self, entry: String) {
        let _ = self.tx.send(SessionMsg::AppendHistory { entry }).await;
    }
}

/// Backward-compatible handle for session store operations.
#[derive(Clone, Debug)]
pub struct SessionStoreActorHandle {
    pub(crate) tx: mpsc::Sender<SessionMsg>,
}

impl SessionStoreActorHandle {
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { tx }
    }

    pub async fn load(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Load { name }).await;
    }

    pub async fn save(&self, name: String, session: Session) {
        let _ = self.tx.send(SessionMsg::Save { name, session }).await;
    }

    pub async fn delete(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Delete { name }).await;
    }

    pub async fn import(&self, path: PathBuf) {
        let _ = self.tx.send(SessionMsg::Import { path }).await;
    }

    pub async fn export(&self, path: PathBuf, session: Session) {
        let _ = self.tx.send(SessionMsg::Export { path, session }).await;
    }

    pub async fn list(&self) {
        let _ = self.tx.send(SessionMsg::List).await;
    }
}
