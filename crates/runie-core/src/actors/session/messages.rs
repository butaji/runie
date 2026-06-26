//! Unified messages and handle for `SessionActor`.
//!
//! Combines responsibilities from the former `PersistenceActor`,
//! `SessionStoreActor`, and `session_actor.rs`.

use std::ops::Deref;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::actors::GenericActorHandle;
use crate::edit_preview::EditPreview;
use crate::session::Session;
use crate::trust::TrustDecision;

// ============================================================================
// Trust + History (formerly PersistenceActor)
// ============================================================================

/// Messages accepted by `SessionActor` — trust and history.
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
// Session State Mutations (formerly direct AppState mutations)
// ============================================================================

/// Messages for session state mutations.
#[derive(Debug, Clone)]
pub enum SessionMutationMsg {
    /// Add a user message with optional image attachments.
    AddUserMessage { content: String, images: Vec<String> },
    /// Add a system message.
    AddSystemMessage { content: String },
    /// Add a tool message.
    AddToolMessage { id: String, name: String, content: String },
    /// Update an existing tool message.
    UpdateToolMessage { id_contains: String, content: String },
    /// Add a turn-complete message.
    AddTurnComplete { id: String, content: String },
    /// Add an error message.
    AddErrorMessage { id: String, content: String },
    /// Reset the session to empty state.
    Reset,
    /// Fork the session tree at the given message index.
    ForkAt { index: usize },
    /// Clone the current branch.
    CloneBranch,
    /// Push a pending edit.
    PushPendingEdit { edit: EditPreview },
    /// Drain all pending edits.
    DrainPendingEdits,
    /// Clear all pending edits.
    ClearPendingEdits,
}

// ============================================================================
// Unified SessionActor
// ============================================================================

/// Unified message enum for the `SessionActor`.
#[derive(Debug, Clone)]
pub enum SessionMsg {
    // Trust + history
    SetTrust {
        path: PathBuf,
        decision: TrustDecision,
    },
    AppendHistory {
        entry: String,
    },
    // Session CRUD
    Load {
        name: String,
    },
    Save {
        name: String,
        session: Session,
    },
    Delete {
        name: String,
    },
    Import {
        path: PathBuf,
    },
    Export {
        path: PathBuf,
        session: Session,
    },
    List,
    // Session state mutations
    AddUserMessage { content: String, images: Vec<String> },
    AddSystemMessage { content: String },
    AddToolMessage { id: String, name: String, content: String },
    UpdateToolMessage { id_contains: String, content: String },
    AddTurnComplete { id: String, content: String },
    AddErrorMessage { id: String, content: String },
    Reset,
    ForkAt { index: usize },
    CloneBranch,
    PushPendingEdit { edit: EditPreview },
    DrainPendingEdits,
    ClearPendingEdits,
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
    inner: GenericActorHandle<SessionMsg>,
}

impl Deref for SessionActorHandle {
    type Target = GenericActorHandle<SessionMsg>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl SessionActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { inner: GenericActorHandle::new(tx) }
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        self.send(SessionMsg::SetTrust { path, decision }).await;
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        self.send(SessionMsg::AppendHistory { entry }).await;
    }

    /// Append an entry to the history file (sync fire-and-forget).
    pub fn try_append_history(&self, entry: String) {
        self.try_send(SessionMsg::AppendHistory { entry });
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        self.send(SessionMsg::Load { name }).await;
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: Session) {
        self.send(SessionMsg::Save { name, session }).await;
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        self.send(SessionMsg::Delete { name }).await;
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: PathBuf) {
        self.send(SessionMsg::Import { path }).await;
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: PathBuf, session: Session) {
        self.send(SessionMsg::Export { path, session }).await;
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        self.send(SessionMsg::List).await;
    }

    // ── Session state mutation methods (fire-and-forget) ──────────────────────

    /// Try to add a user message (fire-and-forget).
    pub fn try_add_user_message(&self, content: String, images: Vec<String>) {
        self.try_send(SessionMsg::AddUserMessage { content, images });
    }

    /// Try to add a system message (fire-and-forget).
    pub fn try_add_system_message(&self, content: String) {
        self.try_send(SessionMsg::AddSystemMessage { content });
    }

    /// Try to add a tool message (fire-and-forget).
    pub fn try_add_tool_message(&self, id: String, name: String, content: String) {
        self.try_send(SessionMsg::AddToolMessage { id, name, content });
    }

    /// Try to update a tool message (fire-and-forget).
    pub fn try_update_tool_message(&self, id_contains: String, content: String) {
        self.try_send(SessionMsg::UpdateToolMessage { id_contains, content });
    }

    /// Try to add a turn-complete message (fire-and-forget).
    pub fn try_add_turn_complete(&self, id: String, content: String) {
        self.try_send(SessionMsg::AddTurnComplete { id, content });
    }

    /// Try to add an error message (fire-and-forget).
    pub fn try_add_error_message(&self, id: String, content: String) {
        self.try_send(SessionMsg::AddErrorMessage { id, content });
    }

    /// Try to reset session (fire-and-forget).
    pub fn try_reset(&self) {
        self.try_send(SessionMsg::Reset);
    }

    /// Try to fork at message index (fire-and-forget).
    pub fn try_fork_at(&self, index: usize) {
        self.try_send(SessionMsg::ForkAt { index });
    }

    /// Try to clone branch (fire-and-forget).
    pub fn try_clone_branch(&self) {
        self.try_send(SessionMsg::CloneBranch);
    }

    /// Try to push a pending edit (fire-and-forget).
    pub fn try_push_pending_edit(&self, edit: EditPreview) {
        self.try_send(SessionMsg::PushPendingEdit { edit });
    }

    /// Try to drain pending edits (fire-and-forget).
    pub fn try_drain_pending_edits(&self) {
        self.try_send(SessionMsg::DrainPendingEdits);
    }

    /// Try to clear pending edits (fire-and-forget).
    pub fn try_clear_pending_edits(&self) {
        self.try_send(SessionMsg::ClearPendingEdits);
    }
}

/// Backward-compatible handle for trust/history operations.
#[derive(Clone, Debug)]
pub struct PersistenceActorHandle {
    inner: GenericActorHandle<SessionMsg>,
}

impl PersistenceActorHandle {
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { inner: GenericActorHandle::new(tx) }
    }

    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        self.inner.send(SessionMsg::SetTrust { path, decision }).await;
    }

    pub async fn append_history(&self, entry: String) {
        self.inner.send(SessionMsg::AppendHistory { entry }).await;
    }
}

/// Backward-compatible handle for session store operations.
#[derive(Clone, Debug)]
pub struct SessionStoreActorHandle {
    inner: GenericActorHandle<SessionMsg>,
}

impl SessionStoreActorHandle {
    pub fn new(tx: mpsc::Sender<SessionMsg>) -> Self {
        Self { inner: GenericActorHandle::new(tx) }
    }

    pub async fn load(&self, name: String) {
        self.inner.send(SessionMsg::Load { name }).await;
    }

    pub async fn save(&self, name: String, session: Session) {
        self.inner.send(SessionMsg::Save { name, session }).await;
    }

    pub async fn delete(&self, name: String) {
        self.inner.send(SessionMsg::Delete { name }).await;
    }

    pub async fn import(&self, path: PathBuf) {
        self.inner.send(SessionMsg::Import { path }).await;
    }

    pub async fn export(&self, path: PathBuf, session: Session) {
        self.inner.send(SessionMsg::Export { path, session }).await;
    }

    pub async fn list(&self) {
        self.inner.send(SessionMsg::List).await;
    }
}
