//! Session state mutation handlers for `SessionActor`.
//!
//! Each handler applies a `SessionMsg` variant to `self.session_state`,
//! bumps the session timestamp, and emits `Event::SessionChanged`.

use crate::edit_preview::EditPreview;
use crate::message::{now, Part};
use crate::model::{ChatMessage, Role, SessionState};
use crate::session::tree::SessionTree;
use crate::Event;

use super::SessionActor;

// ── Session state mutation handlers ──────────────────────────────────────────

impl SessionActor {
    /// Emit `SessionChanged` with the current state snapshot.
    fn emit_changed(&self) {
        let _ = self.bus.publish(Event::SessionChanged {
            state: Box::new(self.session_state.clone()),
        });
    }

    /// Bump session_updated_at to now.
    fn bump_time(&mut self) {
        self.session_state.session_updated_at = now();
    }

    pub(crate) fn handle_add_user_message(&mut self, content: String, images: Vec<String>) {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        self.session_state.image_attachments.extend(images);
        self.session_state.messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_add_system_message(&mut self, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "system".to_owned(),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_add_tool_message(&mut self, id: String, name: String, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            tool_call_id: Some(name),
            ..Default::default()
        });
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_update_tool_message(&mut self, id_contains: &str, content: &str) {
        if let Some(idx) = self
            .session_state
            .messages
            .iter()
            .rposition(|m| m.role == Role::Tool && m.id.contains(id_contains))
        {
            if let Some(msg) = self.session_state.messages.get_mut(idx) {
                msg.set_text_part(content.to_owned());
                msg.timestamp = now();
            }
        }
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_add_turn_complete(&mut self, id: String, content: String) {
        if let Some(idx) = self
            .session_state
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            if let Some(msg) = self.session_state.messages.get_mut(idx) {
                msg.set_text_part(content);
                msg.timestamp = now();
            }
        } else {
            self.session_state.messages.push(ChatMessage {
                role: Role::TurnComplete,
                timestamp: now(),
                id,
                parts: vec![Part::Text { content }],
                ..Default::default()
            });
        }
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_add_error_message(&mut self, id: String, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_reset(&mut self) {
        self.session_state = SessionState::default();
        self.emit_changed();
    }

    pub(crate) fn handle_fork_at(&mut self, index: usize) {
        match self.session_state.session_tree.as_mut() {
            Some(tree) => {
                if let Some(path) = tree.fork_at(index) {
                    tree.navigate_to(&path);
                }
            }
            None => {
                let tree = SessionTree::from_messages(&self.session_state.messages);
                let mut new_tree = tree;
                if let Some(path) = new_tree.fork_at(index) {
                    new_tree.navigate_to(&path);
                }
                self.session_state.session_tree = Some(new_tree);
            }
        }
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_clone_branch(&mut self) {
        let tree = self
            .session_state
            .session_tree
            .clone()
            .unwrap_or_else(|| SessionTree::from_messages(&self.session_state.messages));
        self.session_state.session_tree = Some(tree);
        self.bump_time();
        self.emit_changed();
    }

    pub(crate) fn handle_push_pending_edit(&mut self, edit: EditPreview) {
        self.session_state.pending_edits.push(edit);
        self.emit_changed();
    }

    pub(crate) fn handle_drain_pending_edits(&mut self) {
        self.session_state.pending_edits.clear();
        self.emit_changed();
    }

    pub(crate) fn handle_clear_pending_edits(&mut self) {
        self.session_state.pending_edits.clear();
        self.emit_changed();
    }
}
