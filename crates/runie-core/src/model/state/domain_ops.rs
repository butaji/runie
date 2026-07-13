//! Domain operation methods for `AppState`.
//!
//! These methods implement business logic that was previously in `app_state.rs`.
//! They are kept in a separate file to keep the source files under 500 lines.

use super::ranking;
use super::{AppState, CommandUsage, ModelSource};
use crate::actors::{ConfigMsg, LeaderHandle};
use crate::event::TransientLevel;

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
        // Convert &Path to Utf8PathBuf for lookup (paths are UTF-8 in practice).
        let Some(utf8_path) = camino::Utf8PathBuf::from_path_buf(path.to_path_buf()).ok() else {
            return true; // Non-UTF-8 paths default to trusted.
        };
        match self.trust_decisions_mut().get(&utf8_path) {
            Some(crate::trust::TrustDecision::Trusted) => true,
            Some(crate::trust::TrustDecision::Untrusted) => false,
            None => true, // No decision defaults to trusted.
        }
    }

    pub(crate) fn set_trust_decision(
        &mut self,
        path: camino::Utf8PathBuf,
        decision: crate::trust::TrustDecision,
    ) {
        self.trust_decisions_mut().insert(path, decision);
    }

    /// Set all trust decisions at once (used when loading from persistence).
    pub(crate) fn set_trust_decisions(
        &mut self,
        decisions: indexmap::IndexMap<camino::Utf8PathBuf, crate::trust::TrustDecision>,
    ) {
        *self.trust_decisions_mut() = decisions;
    }

    // ── Actor handles ───────────────────────────────────────────────────────

    /// Install a complete `LeaderHandle` registry.
    pub fn set_actor_handles(&mut self, handles: LeaderHandle) {
        *self.actor_handles_mut() = Some(handles);
    }

    // ── ID generation ───────────────────────────────────────────────────────
    /// Generate next request ID using AppState's own counter.
    ///
    /// This counter is separate from TurnActor's `next_id` to avoid double-increment.
    /// AppState generates IDs for session messages; TurnActor generates IDs for
    /// request queue messages. These are independent — so delivered
    /// steering/follow-up messages (ids from the TurnActor's counter) and
    /// replayed sessions can leave the session holding ids this counter has not
    /// reached yet. Skip any id already present in the session: reissuing one
    /// makes `apply_user_message_submitted` drop the new message as a
    /// "duplicate" and routes the turn's response to the older message.
    pub fn next_id(&mut self) -> String {
        let mut n = self.session_msg_id;
        let mut id = format!("req.{n}");
        while self.session().messages.iter().any(|m| m.id == id) {
            n += 1;
            id = format!("req.{n}");
        }
        self.session_msg_id = n + 1;
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

    /// Braille spinner frame (6-frame cycle) using throbber BRAILLE_SIX symbols
    /// from `crate::labels::BRAILLE_SIX`.
    ///
    /// Wall-clock driven when the turn has a start time (~120ms per frame,
    /// grok cadence — GROK.md §24): the frame derives from elapsed time so the
    /// cadence is independent of the render rate. Falls back to the animation
    /// tick counter when no turn is running (tests, idle previews).
    pub fn spinner_frame(&self) -> char {
        use crate::labels::BRAILLE_SIX;
        const FRAMES: u64 = 6;
        const FRAME_MS: f64 = 120.0;
        let idx = match self.turn_elapsed_secs() {
            Some(elapsed) => ((elapsed * 1000.0 / FRAME_MS) as u64) % FRAMES,
            None => u64::from(self.view().animation_frame % FRAMES as u32),
        };
        BRAILLE_SIX[idx as usize]
    }

    // ── Session reset ──────────────────────────────────────────────────────

    /// Reset session/input/agent state without clearing config,
    /// actor handles, or trust decisions.
    pub fn reset_session(&mut self) {
        let prev = self.take();
        let config = prev.config;
        let actor_handles = prev.actor_handles;
        let event_bus = prev.event_bus;
        let git_info = prev.git_info;
        let cwd_name = prev.cwd_name;
        let trust_decisions = prev.trust_decisions;
        // prev is dropped; all its fields are returned to the pool
        self.config = config;
        *self.actor_handles_mut() = actor_handles;
        self.event_bus = event_bus;
        self.git_info = git_info;
        self.cwd_name = cwd_name;
        *self.trust_decisions_mut() = trust_decisions;
    }

    // ── Config ─────────────────────────────────────────────────────────────

    /// Apply a loaded config to all config-driven state fields.
    pub fn apply_config(&mut self, config: &crate::config::Config) {
        *self.config_mut().model_providers_mut() = config.model_providers.clone();
        // Mirror Config's default provider/model fields so resolve_default_model is unified.
        self.config_mut().provider = config.provider.clone();
        self.config_mut().default_model = config.model.clone();
        if self.config().model_source != ModelSource::UserOverride {
            self.apply_active_model(config);
        }
        self.config_mut().keybindings = crate::keybindings::load_keybindings(Some(config));
        if let Some(theme) = &config.theme {
            self.config_mut().theme_name = theme.clone();
        }
        self.config_mut().truncation = config.truncation.clone();
        self.config_mut().thinking_level = config.thinking_level;
        self.config_mut().model_thinking = config.models.thinking.clone();
        self.config_mut().vim_mode = config.vim_mode();
        let prompts_section = config.prompts();
        *self.prompts_mut() = crate::prompts::load_prompts(
            prompts_section.default.as_deref(),
            prompts_section.custom.as_deref(),
        );
        self.apply_scoped_models(config);
        // Trigger the onboarding/login flow only when there is no usable model.
        // For `--mock-onboarding` the app already has a default mock model, so
        // we also force the flow the first time a config is loaded; after that
        // the session flag prevents re-opening the picker when the saved config
        // reloads.
        let needs_onboarding = !self.onboarding_started
            && ((!self.has_models() && !crate::provider::is_mock_enabled())
                || crate::provider::is_mock_onboarding());
        if needs_onboarding {
            self.onboarding_started = true;
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

    /// List configured providers from ConfigState.
    pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        let mut result: Vec<_> = self
            .config()
            .model_providers()
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Resolve the default provider/model pair from ConfigState.
    ///
    /// Falls back through: explicit `provider` + `default_model`, the
    /// provider's first configured model, the first configured provider's
    /// first model, and finally empty strings. Mirrors
    /// `Config::resolve_default_model` so there is a single source of truth.
    /// The explicit default is preferred over `models[0]` so the active model
    /// does not drift when the stored list is reordered (e.g. sorted on write).
    pub fn resolve_default_model(&self) -> (String, String) {
        if crate::provider::is_mock_enabled() {
            return ("mock".into(), crate::provider::mock_model());
        }
        let cfg = self.config();
        if let Some(provider) = cfg.provider.as_ref().filter(|p| !p.is_empty()) {
            let models: Vec<String> = cfg
                .model_providers
                .get(provider)
                .map(|mp| mp.models.clone())
                .unwrap_or_default();
            // Honor the explicit default when it is a valid choice for this
            // provider; a stale default from another provider is ignored.
            let model = match cfg.default_model.as_deref() {
                Some(def) if models.is_empty() || models.iter().any(|m| m == def) => {
                    def.to_string()
                }
                _ => models.first().cloned().unwrap_or_default(),
            };
            let provider_str = (&provider).to_string();
            return (provider_str, model);
        }
        // Fall back to the first provider in sorted order
        let mut providers: Vec<_> = cfg.model_providers.iter().collect();
        providers.sort_by_key(|(k, _)| *k);
        if let Some((provider, mp)) = providers.into_iter().next() {
            if let Some(model) = mp.models.first().cloned() {
                return (provider.clone(), model);
            }
        }
        (String::new(), String::new())
    }

    /// Look up a configured provider from ConfigState.
    pub fn provider_config(&self, name: &str) -> Option<crate::config::ModelProvider> {
        self.config().model_providers().get(name).cloned()
    }

    /// Fire-and-forget request to remove a provider via ConfigActor.
    pub fn remove_provider(&self, name: &str) {
        if let Some(h) = self.actor_handles() {
            let name = name.to_owned();
            let _ = h.config.try_send(ConfigMsg::RemoveProvider { name });
        }
    }

    /// Fire-and-forget request to update a provider's saved model list.
    pub fn set_provider_models(&self, name: &str, models: Vec<String>) {
        if let Some(h) = self.actor_handles() {
            let name = name.to_owned();
            let _ = h
                .config
                .try_send(ConfigMsg::SetProviderModels { name, models });
        }
    }

    // ── View / render helpers ──────────────────────────────────────────────

    /// Record the height of the message viewport.
    pub fn set_last_visible_height(&mut self, height: u16) {
        self.view_mut().last_visible_height = height;
    }

    /// Record the width of the message content area.
    ///
    /// The caller must pass the already-margined content width — i.e. the
    /// `area.width` value from `f.area().inner(margin)` in ui.rs. This function
    /// stores it as-is; the rendering path (`render_message_content`) subtracts
    /// the remaining 2 cells for left/right glyph margins.
    ///
    /// This ensures the cached line counts and the live render use the same
    /// `content_width` value, avoiding double-subtraction bugs.
    pub fn set_last_content_width(&mut self, width: u16) {
        self.view_mut().last_content_width = width.max(1);
    }

    pub fn cache_generation(&self) -> u64 {
        self.view().message_gen
    }

    /// True when a provider and model are active/connected.
    pub fn has_models(&self) -> bool {
        !self.config().current_provider.is_empty() && !self.config().current_model.is_empty()
    }

    /// True when the view cache needs to be rebuilt.
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
        // Fire-and-forget persist.  In tests without handles, mutation is already applied.
        if let Some(h) = self.actor_handles() {
            let _ = h.config.try_send(ConfigMsg::SetThinkingLevel { level });
        }
        self.notify(
            format!(
                "Thinking level set to: {}",
                self.config().thinking_level.as_str()
            ),
            TransientLevel::Info,
        );
    }

    /// Set or clear the per-model thinking level override (`provider/model`).
    /// Persists to `[models.thinking]` in config.toml via ConfigActor.
    pub fn set_model_thinking_level(
        &mut self,
        provider: &str,
        model: &str,
        level: Option<crate::model::ThinkingLevel>,
    ) {
        let key = format!("{provider}/{model}");
        let current = self.config().model_thinking.get(&key).copied();
        if current == level {
            return;
        }
        match level {
            Some(l) => {
                self.config_mut().model_thinking.insert(key, l);
            }
            None => {
                self.config_mut().model_thinking.remove(&key);
            }
        }
        // Fire-and-forget persist.  In tests without handles, mutation is already applied.
        if let Some(h) = self.actor_handles() {
            let _ = h.config.try_send(ConfigMsg::SetModelThinking {
                provider: provider.to_string(),
                model: model.to_string(),
                level,
            });
        }
    }

    /// The thinking level that applies to the currently active model: the
    /// per-model override when set, otherwise the global level.
    pub fn effective_thinking_level(&self) -> crate::model::ThinkingLevel {
        let key = format!(
            "{}/{}",
            self.config().current_provider,
            self.config().current_model
        );
        self.config()
            .model_thinking
            .get(&key)
            .copied()
            .unwrap_or(self.config().thinking_level)
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

    // ── Session replay helpers ───────────────────────────────────────────────

    /// Restore session metadata (timestamps and display name) from persisted store.
    /// Used by session replay after applying durable events.
    pub fn restore_session_metadata(&mut self, meta: &crate::session::SessionMetadata) {
        self.session_mut().session_created_at = meta.created_at;
        self.session_mut().session_updated_at = meta.updated_at;
        // Only overwrite display_name if it differs from the session id
        // (identical names mean the metadata is storing the session id as fallback)
        if meta.display_name != meta.id {
            self.session_mut().session_display_name = Some(meta.display_name.clone());
        }

        // Restore plan mode if the session has an associated plan
        if let Some(ref plan_id) = meta.active_plan_id {
            if let Some(plans_dir) = crate::session::plan_persistence::default_plans_dir() {
                if let Some(plan) = crate::session::plan_persistence::load_plan(&plans_dir, plan_id)
                    .ok()
                    .flatten()
                {
                    self.view_mut().plan_mode = true;
                    self.view_mut().active_plan_id = Some(plan_id.clone());
                    self.view_mut().active_plan_content = plan.content;
                    tracing::debug!("Restored plan {} for session {}", plan_id, meta.id);
                }
            }
        }
    }

    /// Set session display name (replay helper).
    pub fn set_session_display_name(&mut self, name: Option<String>) {
        self.session_mut().session_display_name = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_default_model_prefers_explicit_default_over_first_model() {
        // Mirror of Config::resolve_default_model (ISSUE H): the domain_ops
        // resolver must honor `default_model` ahead of `models[0]` so the
        // active model does not drift to the lexicographically-first entry.
        crate::provider::set_mock_enabled(false);
        let mut state = AppState::default();
        {
            let cfg = state.config_mut();
            cfg.provider = Some("minimax".to_string());
            cfg.default_model = Some("MiniMax-M2.7".to_string());
            cfg.model_providers.insert(
                "minimax".to_string(),
                crate::config::ModelProvider {
                    provider_type: Some("minimax".to_string()),
                    base_url: "https://api.minimaxi.chat/v1".to_string(),
                    models: vec!["MiniMax-M2".to_string(), "MiniMax-M2.7".to_string()],
                },
            );
        }

        let (provider, model) = state.resolve_default_model();
        assert_eq!(provider, "minimax");
        assert_eq!(
            model, "MiniMax-M2.7",
            "domain_ops resolver must honor default_model over models[0]"
        );

        // Wiring the resolved pair into the active model must not drift.
        state.set_active_model(provider, model, ModelSource::ConfigDefault);
        assert_eq!(state.current_model(), "MiniMax-M2.7");
        assert_ne!(state.current_model(), "MiniMax-M2");
    }
}
