//! Session restore and copy helpers for AppState.

use super::helpers::{element_metadata, element_text};
use super::{AppState, ModelSource};

impl AppState {
    /// Extract plain text from the currently selected post for `y` (copy).
    /// Returns None if no post is selected or if the selection is empty.
    pub fn copy_selected_post_text(&self) -> Option<String> {
        let post_idx = self.view.selected_post?;
        let post = self.view.posts.get(post_idx)?;
        let elements = &self.view.elements_cache;
        let mut lines = Vec::new();
        for i in post.start..post.end {
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
    pub fn copy_selected_post_metadata(&self) -> Option<String> {
        let post_idx = self.view.selected_post?;
        let post = self.view.posts.get(post_idx)?;
        let elements = &self.view.elements_cache;
        let mut parts = Vec::new();
        for i in post.start..post.end.min(elements.len()) {
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
        self.session.messages = session.messages.clone();
        self.set_active_model(
            session.provider.clone(),
            session.model.clone(),
            ModelSource::UserOverride,
        );
        self.config.theme_name = session.theme_name.clone();
        self.config.thinking_level = session.thinking_level;
        self.config.read_only = session.read_only;
        self.session.session_display_name =
            session.display_name.clone().or(Some(session.name.clone()));
        self.session.session_created_at = session.created_at;
        self.session.session_updated_at = session.updated_at;
        self.session.session_tree = session.session_tree.clone();
        self.configure_token_tracker();
        self.messages_changed();
    }

    /// Populate `config_cache` from `login_config` — used by tests that call
    /// `login_config::set_test_config_with_providers` before creating AppState.
    /// Sets config_cache directly without calling apply_config (which would
    /// trigger apply_scoped_models and populate scoped_models from the catalog).
    #[cfg(test)]
    pub fn populate_cache_from_login_config(&mut self) {
        use crate::login_config;
        let providers = login_config::list_configured_providers();
        let mut cfg = crate::config::Config::default();
        for (name, base_url, models) in providers {
            let api_key = login_config::get_provider_config(&name)
                .map(|(_, k, _)| k)
                .unwrap_or_default();
            cfg.model_providers.insert(
                name.clone(),
                crate::config::ModelProvider {
                    provider_type: None,
                    base_url,
                    api_key,
                    models,
                },
            );
        }
        if !cfg.model_providers.is_empty() {
            self.config_cache = Some(cfg);
        }
    }
}
