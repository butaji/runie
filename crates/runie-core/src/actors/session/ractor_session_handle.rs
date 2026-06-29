//! Ractor-based SessionActor handle.

use std::path::PathBuf;

use crate::actors::ractor_adapter::RactorHandle;
use crate::edit_preview::EditPreview;
use crate::session::Session;
use crate::trust::TrustDecision;

use super::messages::SessionMsg;

/// Ractor-based SessionActor handle.
#[derive(Clone, Debug)]
pub struct RactorSessionHandle {
    inner: RactorHandle<SessionMsg>,
}

impl RactorSessionHandle {
    pub fn new(inner: RactorHandle<SessionMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: SessionMsg) {
        self.inner.send(msg).await;
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        self.inner.send(SessionMsg::SetTrust { path, decision }).await;
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        self.inner.send(SessionMsg::AppendHistory { entry }).await;
    }

    /// Append an entry to the history file (sync fire-and-forget).
    pub fn try_append_history(&self, entry: String) {
        let _ = self.inner.try_send(SessionMsg::AppendHistory { entry });
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        self.inner.send(SessionMsg::Load { name }).await;
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: Session) {
        self.inner.send(SessionMsg::Save { name, session }).await;
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        self.inner.send(SessionMsg::Delete { name }).await;
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        self.inner.send(SessionMsg::List).await;
    }

    /// Try to add a user message (fire-and-forget).
    pub fn try_add_user_message(&self, content: String, images: Vec<String>) {
        let _ = self.inner.try_send(SessionMsg::AddUserMessage { content, images });
    }

    /// Try to add a system message (fire-and-forget).
    pub fn try_add_system_message(&self, content: String) {
        let _ = self.inner.try_send(SessionMsg::AddSystemMessage { content });
    }

    /// Try to add a tool message (fire-and-forget).
    pub fn try_add_tool_message(&self, id: String, name: String, content: String) {
        let _ = self.inner.try_send(SessionMsg::AddToolMessage { id, name, content });
    }

    /// Try to update a tool message (fire-and-forget).
    pub fn try_update_tool_message(&self, id_contains: String, content: String) {
        let _ = self.inner.try_send(SessionMsg::UpdateToolMessage { id_contains, content });
    }

    /// Try to add a turn-complete message (fire-and-forget).
    pub fn try_add_turn_complete(&self, id: String, content: String) {
        let _ = self.inner.try_send(SessionMsg::AddTurnComplete { id, content });
    }

    /// Try to add an error message (fire-and-forget).
    pub fn try_add_error_message(&self, id: String, content: String) {
        let _ = self.inner.try_send(SessionMsg::AddErrorMessage { id, content });
    }

    /// Try to reset session (fire-and-forget).
    pub fn try_reset(&self) {
        let _ = self.inner.try_send(SessionMsg::Reset);
    }

    /// Try to fork at message index (fire-and-forget).
    pub fn try_fork_at(&self, index: usize) {
        let _ = self.inner.try_send(SessionMsg::ForkAt { index });
    }

    /// Try to clone branch (fire-and-forget).
    pub fn try_clone_branch(&self) {
        let _ = self.inner.try_send(SessionMsg::CloneBranch);
    }

    /// Try to push a pending edit (fire-and-forget).
    pub fn try_push_pending_edit(&self, edit: EditPreview) {
        let _ = self.inner.try_send(SessionMsg::PushPendingEdit { edit });
    }

    /// Try to drain pending edits (fire-and-forget).
    pub fn try_drain_pending_edits(&self) {
        let _ = self.inner.try_send(SessionMsg::DrainPendingEdits);
    }

    /// Try to clear pending edits (fire-and-forget).
    pub fn try_clear_pending_edits(&self) {
        let _ = self.inner.try_send(SessionMsg::ClearPendingEdits);
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: PathBuf) {
        self.inner.send(SessionMsg::Import { path }).await;
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: PathBuf, session: Session) {
        self.inner.send(SessionMsg::Export { path, session }).await;
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send_message(&self, msg: SessionMsg) {
        self.inner.send(msg).await;
    }
}
