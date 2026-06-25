//! Domain operation methods for `AppState`.
//!
//! These methods implement business logic that was previously in `app_state.rs`.
//! They are kept in a separate file to keep the source files under 500 lines.

use super::ranking;
use super::{AppState, CommandUsage, ModelSource};
use crate::actors::ActorHandles;
use crate::event::TransientLevel;
use crate::view::elements::Element;

impl AppState {
    // ── Initialization setters ──────────────────────────────────────────────

    /// Set git info from bootstrap (file I/O off the async runtime).
    pub fn set_git_info(&mut self, git_info: Option<crate::snapshot::GitInfo>) {
        *self.git_info_mut() = git_info;
    }

    /// Set the current working directory name from bootstrap.
    pub fn set_cwd_name(&mut self, cwd_name: String) {
        *self.cwd_name_mut() = cwd_name;
    }

    /// Set the loaded skills from bootstrap.
    pub fn set_skills(&mut self, skills: Vec<crate::skills::Skill>) {
        *self.skills_mut() = skills;
    }

    // ── Login flow helpers ──────────────────────────────────────────────────

    /// Returns the provider from the active login flow, or the config's current provider.
    pub fn active_provider(&self) -> String {
        self.login_flow
            .as_ref()
            .map(|f| f.provider.clone())
            .unwrap_or_else(|| self.config().current_provider.clone())
    }

    // ── Trust ────────────────────────────────────────────────────────────────

    pub fn is_trusted(&mut self, path: &std::path::Path) -> bool {
        match self.trust_decisions_mut().get(path) {
            Some(crate::trust::TrustDecision::Trusted) | None => true,
            Some(crate::trust::TrustDecision::Untrusted) => false,
        }
    }

    pub(crate) fn set_trust_decision(
        &mut self,
        path: std::path::PathBuf,
        decision: crate::trust::TrustDecision,
    ) {
        self.trust_decisions_mut().insert(path, decision);
    }

    // ── Actor handles ───────────────────────────────────────────────────────

    /// Install a complete `ActorHandles` registry.
    pub fn set_actor_handles(&mut self, handles: ActorHandles) {
        *self.actor_handles_mut() = Some(handles);
    }

    // ── Input history ──────────────────────────────────────────────────────

    pub fn add_to_input_history(&mut self, entry: String) {
        self.input_mut().input_history.retain(|h| h != &entry);
        self.input_mut().input_history.push(entry);
    }

    // ── ID generation ───────────────────────────────────────────────────────

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.agent_state().next_id);
        self.agent_state_mut().next_id += 1;
        id
    }

    // ── Message/view dirty tracking ─────────────────────────────────────────

    /// Call when the message list changes to bump view generation and session
    /// timestamp. This is the canonical way to invalidate the element cache.
    pub fn messages_changed(&mut self) {
        self.view_mut().message_gen = self.view().message_gen.wrapping_add(1);
        self.session_mut().session_updated_at = crate::message::now();
        self.view_mut().dirty = true;
    }

    /// Call after the message list changes: bumps generation and rebuilds caches.
    /// Combines `messages_changed()` + `ensure_fresh()` for the common test pattern.
    pub fn refresh_after_message_change(&mut self) {
        self.messages_changed();
        self.ensure_fresh();
    }

    // ── Turn lifecycle ──────────────────────────────────────────────────────

    /// Returns whether a turn is currently active.
    pub fn turn_active(&self) -> bool {
        self.agent_state().turn_active
    }

    /// Start a new agent turn.
    pub fn start_turn(&mut self) {
        self.agent_state_mut().turn_active = true;
        self.agent_state_mut().inflight += 1;
        self.agent_state_mut().streaming = true;
    }

    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.agent_state()
            .thinking_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.agent_state()
            .turn_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.agent_state()
            .tool_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        const SPINNER_CHARS: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
        const SPINNER_FRAMES: u32 = 12;
        SPINNER_CHARS[(self.view().animation_frame % SPINNER_FRAMES) as usize]
    }

    // ── Session reset ───────────────────────────────────────────────────────

    /// Reset session/input/agent state without clearing config,
    /// actor handles, or trust decisions.
    pub fn reset_session(&mut self) {
        let prev = self.take();
        let config = prev.config;
        let actor_handles = prev.actor_handles;
        let config_cache = prev.config_cache;
        let git_info = prev.git_info;
        let cwd_name = prev.cwd_name;
        let trust_decisions = prev.trust_decisions;
        // prev is dropped; all its fields are returned to the pool
        self.config = config;
        *self.actor_handles_mut() = actor_handles;
        *self.config_cache_mut() = config_cache;
        self.git_info = git_info;
        self.cwd_name = cwd_name;
        *self.trust_decisions_mut() = trust_decisions;
    }

    // ── Config ─────────────────────────────────────────────────────────────

    /// Apply a loaded config to all config-driven state fields.
    pub fn apply_config(&mut self, config: &crate::config::Config) {
        *self.config_cache_mut() = Some(config.clone());
        if self.config().model_source != ModelSource::UserOverride {
            self.apply_active_model(config);
        }
        self.config_mut().keybindings = crate::keybindings::load_keybindings(Some(config));
        if let Some(theme) = &config.theme {
            self.config_mut().theme_name = theme.clone();
        }
        self.config_mut().truncation = config.truncation.clone();
        self.config_mut().thinking_level = config.thinking_level;
        self.config_mut().vim_mode = config.vim_mode();
        self.config_mut().telemetry = crate::telemetry::Telemetry::new(config.telemetry_enabled());
        let prompts_section = config.prompts();
        *self.prompts_mut() = crate::prompts::load_prompts(
            prompts_section.default.as_deref(),
            prompts_section.custom.as_deref(),
        );
        self.apply_scoped_models(config);
        if !self.has_models() && !crate::provider::is_mock_enabled() {
            self.update(crate::Event::Start);
        }
    }

    fn apply_active_model(&mut self, config: &crate::config::Config) {
        let (provider, model) = config.resolve_default_model();
        if !provider.is_empty() && ranking::has_provider_credentials(config, &provider) {
            self.set_active_model(provider, model, ModelSource::ConfigDefault);
        }
    }

    fn apply_scoped_models(&mut self, config: &crate::config::Config) {
        if let Some(scoped) = config.scoped_models() {
            self.config_mut().scoped_models =
                scoped.iter().map(|s| self.parse_scoped_model(s)).collect();
        } else {
            self.config_mut().scoped_models = crate::model_catalog::model_catalog()
                .iter()
                .take(10)
                .map(|m| crate::model::ScopedModel {
                    provider: m.provider.clone(),
                    name: m.name.clone(),
                    enabled: true,
                })
                .collect();
        }
    }

    fn parse_scoped_model(&self, s: &str) -> crate::model::ScopedModel {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            crate::model::ScopedModel {
                provider: parts[0].to_owned(),
                name: parts[1].to_owned(),
                enabled: true,
            }
        } else {
            crate::model::ScopedModel {
                provider: self.config().current_provider.clone(),
                name: s.to_owned(),
                enabled: true,
            }
        }
    }

    /// List configured providers from the cached config.
    pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        self.config_cache
            .as_ref()
            .map(|c| c.configured_providers())
            .unwrap_or_default()
    }

    /// Resolve the default provider/model pair from the cached config.
    pub fn resolve_default_model(&self) -> (String, String) {
        self.config_cache
            .as_ref()
            .map(|c| c.resolve_default_model())
            .unwrap_or_default()
    }

    /// Look up a configured provider from the cached config.
    pub fn provider_config(&self, name: &str) -> Option<crate::config::ModelProvider> {
        self.config_cache
            .as_ref()
            .and_then(|c| c.model_providers.get(name).cloned())
    }

    /// Fire-and-forget request to remove a provider via ConfigActor.
    pub fn remove_provider(&self, name: &str) {
        let tx = self
            .actor_handles
            .as_ref()
            .and_then(|h| h.config.as_ref())
            .map(|h| h.tx().clone());
        if let (Some(tx), Ok(_)) = (tx, tokio::runtime::Handle::try_current()) {
            let msg = crate::actors::ConfigMsg::RemoveProvider {
                name: name.to_owned(),
            };
            tokio::spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
    }

    /// Fire-and-forget request to update a provider's saved model list.
    pub fn set_provider_models(&self, name: &str, models: Vec<String>) {
        let tx = self
            .actor_handles
            .as_ref()
            .and_then(|h| h.config.as_ref())
            .map(|h| h.tx().clone());
        if let (Some(tx), Ok(_)) = (tx, tokio::runtime::Handle::try_current()) {
            let msg = crate::actors::ConfigMsg::SetProviderModels {
                name: name.to_owned(),
                models,
            };
            tokio::spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
    }

    // ── View / render helpers ──────────────────────────────────────────────

    /// Record the height of the message viewport.
    pub fn set_last_visible_height(&mut self, height: u16) {
        self.view_mut().last_visible_height = height;
    }

    /// Record the width of the message content area.
    pub fn set_last_content_width(&mut self, width: u16) {
        self.view_mut().last_content_width = width;
    }

    pub fn cache_generation(&self) -> u64 {
        self.view().message_gen
    }

    /// True when a provider and model are active/connected.
    pub fn has_models(&self) -> bool {
        !self.config().current_provider.is_empty() && !self.config().current_model.is_empty()
    }

    /// Visible elements slice — O(1), zero allocation.
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        crate::snapshot::visible_slice(&self.view().elements_cache, skip, take)
    }

    pub fn count(&self) -> usize {
        self.view()
            .element_count
            .max(self.view().elements_cache.len())
    }

    pub fn element_count(&self) -> usize {
        self.view().element_count
    }

    pub fn total_lines(&self) -> usize {
        self.view().total_lines
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        crate::snapshot::scroll_offset(self.view().total_lines, self.view().scroll, visible_height)
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        crate::snapshot::scrollbar_metrics(
            self.view().total_lines,
            self.view().scroll,
            visible_height,
        )
    }

    pub fn elements_cache(&self) -> &[Element] {
        self.view().elements_cache.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.view().dirty
    }

    // ── Model / command usage ───────────────────────────────────────────────

    /// Record a model selection in recent history (max 5, no duplicates).
    pub fn record_model_usage(&mut self, provider: &str, model: &str) {
        let full = format!("{}/{}", provider, model);
        self.config_mut().recent_models.retain(|m| m != &full);
        self.config_mut().recent_models.push(full);
        if self.config_mut().recent_models.len() > 5 {
            self.config_mut().recent_models.remove(0);
        }
    }

    /// Record that a command was invoked for palette ranking.
    pub fn record_command_usage(&mut self, name: &str) {
        let now = crate::update::now();
        let entry = self
            .config
            .command_usage
            .entry(name.to_owned())
            .or_insert_with(|| CommandUsage {
                count: 0,
                last_used: now,
            });
        entry.count += 1;
        entry.last_used = now;
    }

    /// Rank commands by fuzzy match score, recency boost, and usage count.
    /// Returns commands in ranked order, limited to `limit`.
    pub fn rank_commands(
        &mut self,
        query: &str,
        limit: usize,
    ) -> Vec<(&crate::commands::CommandDef, i32)> {
        let command_usage = self.config().command_usage.clone();
        let all: Vec<_> = self.registry_mut().list();
        let ranked_names: Vec<(String, i32)> = if query.is_empty() {
            ranking::rank_commands_empty_query(&command_usage, &all, limit)
        } else {
            ranking::rank_commands_with_query(&command_usage, query, &all, limit)
        };
        ranked_names
            .into_iter()
            .filter_map(|(name, score)| self.registry().get(&name).map(|cmd| (cmd, score)))
            .collect()
    }

    // ── Config read helpers (for external crates) ───────────────────────────

    /// Returns the current prompt name configured in the input state.
    pub fn current_prompt(&self) -> &str {
        &self.input().current_prompt
    }

    /// Returns the current provider ID.
    pub fn current_provider(&self) -> &str {
        &self.config().current_provider
    }

    /// Returns the current model name.
    pub fn current_model(&self) -> &str {
        &self.config().current_model
    }

    /// Returns the current thinking level setting.
    pub fn thinking_level(&self) -> crate::model::ThinkingLevel {
        self.config().thinking_level
    }

    /// Set the thinking level and update derived state.
    /// Persists to config.toml via ConfigActor.
    pub(crate) fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        if self.config().thinking_level == level {
            return;
        }
        self.config_mut().thinking_level = level;
        let handles = self.actor_handles().cloned();
        if let Some(h) = handles {
            if tokio::runtime::Handle::try_current().is_ok() {
                let h = h;
                tokio::spawn(async move {
                    h.send_set_thinking_level(level).await;
                });
            }
        }
        self.notify(
            format!(
                "Thinking level set to: {}",
                self.config().thinking_level.as_str()
            ),
            TransientLevel::Info,
        );
    }

    /// Returns whether the app is in read-only mode.
    pub fn read_only(&self) -> bool {
        self.config().read_only
    }

    /// Current vim_mode setting.
    pub fn vim_mode(&self) -> bool {
        self.config().vim_mode
    }

    /// Returns the truncation configuration for tool output.
    pub fn truncation(&self) -> &crate::config::TruncationSection {
        &self.config().truncation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_provider_returns_login_flow_provider() {
        let mut state = AppState::default();
        *state.login_flow_mut() = Some(crate::login_flow::LoginFlowState {
            step: crate::login_flow::LoginStep::KeyInput,
            provider: "anthropic".to_string(),
            key: "sk-test".to_string(),
            available_models: vec![],
            selected_models: std::collections::HashSet::new(),
            validated: false,
        });
        assert_eq!(state.active_provider(), "anthropic");
    }

    #[test]
    fn active_provider_returns_config_default_when_no_flow() {
        let mut state = AppState::default();
        state.config_mut().current_provider = "openai".to_string();
        *state.login_flow_mut() = None;
        assert_eq!(state.active_provider(), "openai");
    }

    #[test]
    fn active_provider_returns_config_default_when_no_flow_no_config() {
        // In test mode, default ConfigState sets current_provider to "mock"
        let mut state = AppState::default();
        *state.login_flow_mut() = None;
        // active_provider falls back to config.current_provider
        assert_eq!(state.active_provider(), "mock");
    }
}
