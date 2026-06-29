//! Unified messages for `SessionActor`.
//!
//! Combines responsibilities from the former `PersistenceActor`,
//! `SessionStoreActor`, and `session_actor.rs`.

use std::path::PathBuf;

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
    AddUserMessage {
        content: String,
        images: Vec<String>,
    },
    /// Add a system message.
    AddSystemMessage { content: String },
    /// Add a tool message.
    AddToolMessage {
        id: String,
        name: String,
        content: String,
    },
    /// Update an existing tool message.
    UpdateToolMessage {
        id_contains: String,
        content: String,
    },
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
    AddUserMessage {
        content: String,
        images: Vec<String>,
    },
    AddSystemMessage {
        content: String,
    },
    AddToolMessage {
        id: String,
        name: String,
        content: String,
    },
    UpdateToolMessage {
        id_contains: String,
        content: String,
    },
    AddTurnComplete {
        id: String,
        content: String,
    },
    AddErrorMessage {
        id: String,
        content: String,
    },
    Reset,
    ForkAt {
        index: usize,
    },
    CloneBranch,
    PushPendingEdit {
        edit: EditPreview,
    },
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
