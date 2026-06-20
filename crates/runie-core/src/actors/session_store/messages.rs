//! Messages and handle for `SessionStoreActor`.

use tokio::sync::mpsc;

/// Messages accepted by `SessionStoreActor`.
#[derive(Debug, Clone)]
pub enum SessionStoreMsg {
    /// Load a named session and publish its durable events.
    Load { name: String },
    /// Save a session snapshot under the given name.
    Save { name: String, session: crate::session::Session },
    /// Delete a named session.
    Delete { name: String },
    /// Import a session snapshot from a file path.
    Import { path: std::path::PathBuf },
    /// Export a session snapshot to a file path.
    Export { path: std::path::PathBuf, session: crate::session::Session },
    /// List all saved sessions.
    List,
}

/// Handle for sending commands to a `SessionStoreActor`.
#[derive(Clone, Debug)]
pub struct SessionStoreActorHandle {
    tx: mpsc::Sender<SessionStoreMsg>,
}

impl SessionStoreActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<SessionStoreMsg>) -> Self {
        Self { tx }
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        let _ = self.tx.send(SessionStoreMsg::Load { name }).await;
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: crate::session::Session) {
        let _ = self.tx.send(SessionStoreMsg::Save { name, session }).await;
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        let _ = self.tx.send(SessionStoreMsg::Delete { name }).await;
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: std::path::PathBuf) {
        let _ = self.tx.send(SessionStoreMsg::Import { path }).await;
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: std::path::PathBuf, session: crate::session::Session) {
        let _ = self.tx.send(SessionStoreMsg::Export { path, session }).await;
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        let _ = self.tx.send(SessionStoreMsg::List).await;
    }
}
