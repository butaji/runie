//! Session restore and copy helpers for AppState.

use super::helpers::{element_metadata, element_text};
use super::{AppState, ModelSource};

impl AppState {
    /// Extract plain text from the currently selected post for `y` (copy).
    /// Returns None if no post is selected or if the selection is empty.
    pub fn copy_selected_post_text(&mut self) -> Option<String> {
        let post_idx = self.view_mut().selected_post?;
        let (start, end) = {
            let post = self.view_mut().posts.get(post_idx)?;
            (post.start, post.end)
        };
        let elements = &self.view_mut().elements_cache;
        let mut lines = Vec::new();
        for i in start..end {
            if let Some(elem) = elements.get(i) {
                if let Some(text) = element_text(elem) {
                    lines.push(text);
                }
            }
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Extract metadata from the currently selected post for `Y` (copy metadata).
    pub fn copy_selected_post_metadata(&mut self) -> Option<String> {
        let post_idx = self.view_mut().selected_post?;
        let (start, end) = {
            let post = self.view_mut().posts.get(post_idx)?;
            (post.start, post.end)
        };
        let elements = &self.view_mut().elements_cache;
        let mut parts = Vec::new();
        for i in start..end.min(elements.len()) {
            if let Some(elem) = elements.get(i) {
                if let Some(meta) = element_metadata(elem) {
                    parts.push(meta);
                }
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }

    /// Restore application state from a JSON session snapshot.
    pub fn restore_session(&mut self, session: &crate::session::Session) {
        self.session_mut().messages = session.messages.clone();
        self.set_active_model(
            session.provider.clone(),
            session.model.clone(),
            ModelSource::UserOverride,
        );
        self.config_mut().theme_name = session.theme_name.clone();
        self.config_mut().thinking_level = session.thinking_level;
        self.config_mut().read_only = session.read_only;
        self.session_mut().session_display_name =
            session.display_name.clone().or(Some(session.name.clone()));
        self.session_mut().session_created_at = session.created_at;
        self.session_mut().session_updated_at = session.updated_at;
        self.session_mut().session_tree = session.session_tree.clone();
        self.configure_token_tracker();
        self.messages_changed();
    }

}
