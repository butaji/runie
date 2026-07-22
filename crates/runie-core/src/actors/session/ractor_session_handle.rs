//! Ractor-based SessionActor handle.

use camino::Utf8PathBuf;
use std::path::PathBuf;

use crate::edit_preview::EditPreview;
use crate::session::Session;
use crate::trust::TrustDecision;
use ractor::ActorRef;

use super::messages::SessionMsg;

/// Ractor-based SessionActor handle.
#[derive(Clone, Debug)]
pub struct RactorSessionHandle {
    inner: ActorRef<SessionMsg>,
}

impl RactorSessionHandle {
    pub fn new(inner: ActorRef<SessionMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: SessionMsg) {
        let _ = self.inner.send_message(msg);
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        let path_utf8 = Utf8PathBuf::from_path_buf(path).unwrap_or_else(|_| Utf8PathBuf::from("."));
        let _ = self
            .inner
            .send_message(SessionMsg::SetTrust { path: path_utf8, decision });
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        let _ = self.inner.send_message(SessionMsg::AppendHistory { entry });
    }

    /// Append an entry to the history file (sync fire-and-forget).
    pub fn try_append_history(&self, entry: String) {
        let _ = self.inner.send_message(SessionMsg::AppendHistory { entry });
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        let _ = self.inner.send_message(SessionMsg::Load { name });
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: Session) {
        let _ = self.inner.send_message(SessionMsg::Save { name, session });
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        let _ = self.inner.send_message(SessionMsg::Delete { name });
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        let _ = self.inner.send_message(SessionMsg::List);
    }

    /// Request resuming the most recent session.
    pub async fn resume_most_recent(&self) {
        let _ = self.inner.send_message(SessionMsg::ResumeMostRecent);
    }

    /// Try to request resuming the most recent session (fire-and-forget).
    pub fn try_resume_most_recent(&self) {
        let _ = self.inner.send_message(SessionMsg::ResumeMostRecent);
    }

    /// Try to add a user message (fire-and-forget).
    pub fn try_add_user_message(&self, content: String, images: Vec<String>) {
        let _ = self
            .inner
            .send_message(SessionMsg::AddUserMessage { content, images });
    }

    /// Try to add a system message (fire-and-forget).
    pub fn try_add_system_message(&self, content: String) {
        let _ = self
            .inner
            .send_message(SessionMsg::AddSystemMessage { content });
    }

    /// Try to add a tool message (fire-and-forget).
    pub fn try_add_tool_message(&self, id: String, name: String, content: String) {
        let _ = self
            .inner
            .send_message(SessionMsg::AddToolMessage { id, name, content });
    }

    /// Try to update a tool message (fire-and-forget).
    pub fn try_update_tool_message(&self, id_contains: String, content: String) {
        let _ = self
            .inner
            .send_message(SessionMsg::UpdateToolMessage { id_contains, content });
    }

    /// Try to add a turn-complete message (fire-and-forget).
    pub fn try_add_turn_complete(&self, id: String, content: String) {
        let _ = self
            .inner
            .send_message(SessionMsg::AddTurnComplete { id, content });
    }

    /// Try to add an error message (fire-and-forget).
    pub fn try_add_error_message(&self, id: String, content: String) {
        let _ = self
            .inner
            .send_message(SessionMsg::AddErrorMessage { id, content });
    }

    /// Try to reset session (fire-and-forget).
    pub fn try_reset(&self) {
        let _ = self.inner.send_message(SessionMsg::Reset);
    }

    /// Try to fork at message index (fire-and-forget).
    pub fn try_fork_at(&self, index: usize) {
        let _ = self.inner.send_message(SessionMsg::ForkAt { index });
    }

    /// Try to clone branch (fire-and-forget).
    pub fn try_clone_branch(&self) {
        let _ = self.inner.send_message(SessionMsg::CloneBranch);
    }

    /// Try to push a pending edit (fire-and-forget).
    pub fn try_push_pending_edit(&self, edit: EditPreview) {
        let _ = self
            .inner
            .send_message(SessionMsg::PushPendingEdit { edit });
    }

    /// Try to drain pending edits (fire-and-forget).
    pub fn try_drain_pending_edits(&self) {
        let _ = self.inner.send_message(SessionMsg::DrainPendingEdits);
    }

    /// Try to clear pending edits (fire-and-forget).
    pub fn try_clear_pending_edits(&self) {
        let _ = self.inner.send_message(SessionMsg::ClearPendingEdits);
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: PathBuf) {
        let _ = self.inner.send_message(SessionMsg::Import { path });
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: PathBuf, session: Session) {
        let _ = self
            .inner
            .send_message(SessionMsg::Export { path, session });
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: SessionMsg) -> Result<(), Box<ractor::MessagingErr<SessionMsg>>> {
        self.inner.send_message(msg).map_err(Box::new)
    }
}
