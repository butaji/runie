//! `AppState` struct and its inherent methods.
use super::helpers::{compute_ranking_score, element_metadata, element_text};
use super::FffFileEntry;
use crate::ui::elements::Element;

#[derive(Clone)]
pub struct AppState {
    // 6 inner state structs (factored domain state)
    pub session: crate::state::SessionState,
    pub input: crate::state::InputState,
    pub agent: crate::state::AgentState,
    pub view: crate::state::ViewState,
    pub config: crate::state::ConfigState,
    pub completion: crate::state::CompletionState,

    // Singleton UI/control flags (don't fit a single domain)
    /// Quit flag read by the main event loop
    pub should_quit: bool,
    /// Currently open overlay dialog (palette, model selector, etc.)
    pub open_dialog: Option<crate::commands::DialogState>,
    /// Stack for nested dialog navigation (Esc pops, restoring parent)
    pub dialog_back_stack: Vec<crate::commands::DialogState>,
    /// Active login/auth flow overlay
    pub login_flow: Option<crate::login_flow::LoginFlowState>,
    /// Command registry (loaded once, immutable per session)
    pub registry: crate::commands::CommandRegistry,
    /// Loaded skill definitions
    pub skills: Vec<crate::skills::Skill>,
    /// Loaded prompt templates
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    /// Transient notification message (cleared after timeout)
    pub transient_message: Option<String>,
    pub transient_until: Option<std::time::Instant>,
    pub transient_level: Option<crate::event::TransientLevel>,
    /// Git info detected at startup (repo name, branch)
    pub git_info: Option<crate::snapshot::GitInfo>,
    /// Current working directory name (detected at startup)
    pub cwd_name: String,
    /// FFF search results for the current file picker query.
    /// Set when FFF indexer returns results (populated asynchronously).
    pub fff_file_results: Vec<FffFileEntry>,
    /// Counter incremented each time the user types in the file picker.
    /// Used to detect stale FFF results (result counter != current counter means ignore).
    pub fff_debounce: u32,
    /// Active permission approval prompt (blocking modal dialog).
    pub permission_request: Option<crate::model::PermissionRequestState>,
    /// Cross-actor coordination registry for in-flight permission approvals.
    ///
    /// The agent turn registers a oneshot here and blocks; the UI resolves it
    /// when the user chooses an action. This shared registry is deliberate
    /// request/response plumbing between the agent task and the UI task, not
    /// accidental global mutable state.
    pub approval_registry: std::sync::Arc<std::sync::Mutex<crate::permissions::ApprovalRegistry>>,
    /// Sender to the `ConfigActor`. `None` in unit tests that do not spawn it.
    pub config_tx: Option<tokio::sync::mpsc::Sender<crate::actors::ConfigMsg>>,
    /// Sender to the `ProviderActor`. `None` in unit tests that do not spawn it.
    pub provider_tx: Option<tokio::sync::mpsc::Sender<crate::actors::ProviderMsg>>,
    /// Last config applied to the state (read-only cache for sync lookups).
    pub config_cache: Option<crate::config::Config>,
}

impl Default for AppState {
    fn default() -> Self {
        let (git_info, cwd_name) = crate::model::init_git_and_cwd();
        Self {
            session: crate::state::SessionState::default(),
            input: crate::state::InputState::default(),
            agent: crate::state::AgentState::default(),
            view: crate::state::ViewState::default(),
            config: crate::state::ConfigState::default(),
            completion: crate::state::CompletionState::default(),
            should_quit: false,
            open_dialog: None,
            dialog_back_stack: Vec::new(),
            login_flow: None,
            registry: crate::commands::CommandRegistry::new(),
            skills: Vec::new(),
            prompts: Vec::new(),
            transient_message: None,
            transient_until: None,
            transient_level: None,
            git_info,
            cwd_name,
            fff_file_results: Vec::new(),
            fff_debounce: 0,
            permission_request: None,
            approval_registry: std::sync::Arc::new(std::sync::Mutex::new(
                crate::permissions::ApprovalRegistry::new(),
            )),
            config_tx: None,
            provider_tx: None,
            config_cache: None,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .thinking_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .turn_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .tool_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        const SPINNER_CHARS: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
        const SPINNER_FRAMES: u32 = 12;
        SPINNER_CHARS[(self.view.animation_frame % SPINNER_FRAMES) as usize]
    }

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.agent.next_id);
        self.agent.next_id += 1;
        id
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.view.dirty = true;
    }

    pub fn messages_changed(&mut self) {
        self.view.message_gen = self.view.message_gen.wrapping_add(1);
        self.session.session_updated_at = crate::message::now();
        self.view.dirty = true;
    }

    /// Reset session/input/agent state without clearing the connected provider/model.
    pub fn reset_session(&mut self) {
        let config = self.config.clone();
        let registry = self.approval_registry.clone();
        let config_tx = self.config_tx.clone();
        let provider_tx = self.provider_tx.clone();
        let config_cache = self.config_cache.clone();
        *self = Self::default();
        self.config = config;
        self.approval_registry = registry;
        self.config_tx = config_tx;
        self.provider_tx = provider_tx;
        self.config_cache = config_cache;
    }

    /// Apply a loaded config to all config-driven state fields.
    pub fn apply_config(&mut self, config: &crate::config::Config) {
        self.config_cache = Some(config.clone());
        if self.config.model_source != crate::state::ModelSource::UserOverride {
            self.apply_active_model(config);
        }
        self.config.keybindings = crate::keybindings::load_keybindings(Some(config));
        if let Some(theme) = &config.theme {
            self.config.theme_name = theme.clone();
        }
        self.config.truncation = config.truncation.clone();
        self.config.vim_mode = config.vim_mode();
        self.config.telemetry = crate::telemetry::Telemetry::new(config.telemetry_enabled());
        let prompts_section = config.prompts();
        self.prompts = crate::prompts::load_prompts(
            prompts_section.default.as_deref(),
            prompts_section.custom.as_deref(),
        );
        self.apply_scoped_models(config);
        if !self.has_models() && !crate::provider_registry::is_mock_enabled() {
            self.update(crate::event::LoginFlowEvent::Start);
        }
    }

    fn apply_active_model(&mut self, config: &crate::config::Config) {
        let (provider, model) = config.resolve_default_model();
        if !provider.is_empty() && has_provider_credentials(config, &provider) {
            self.set_active_model(provider, model, crate::state::ModelSource::ConfigDefault);
        }
    }

    fn apply_scoped_models(&mut self, config: &crate::config::Config) {
        if let Some(scoped) = config.scoped_models() {
            self.config.scoped_models = scoped.iter().map(|s| self.parse_scoped_model(s)).collect();
        } else {
            self.config.scoped_models = crate::model_catalog::model_catalog()
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
                provider: parts[0].to_string(),
                name: parts[1].to_string(),
                enabled: true,
            }
        } else {
            crate::model::ScopedModel {
                provider: self.config.current_provider.clone(),
                name: s.to_string(),
                enabled: true,
            }
        }
    }

    /// List configured providers from the cached config.
    pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        if let Some(config) = self.config_cache.as_ref() {
            return config.configured_providers();
        }
        #[cfg(test)]
        {
            return crate::login_config::list_configured_providers();
        }
        #[cfg(not(test))]
        Vec::new()
    }

    /// Resolve the default provider/model pair from the cached config.
    pub fn resolve_default_model(&self) -> (String, String) {
        if let Some(config) = self.config_cache.as_ref() {
            return config.resolve_default_model();
        }
        #[cfg(test)]
        {
            return crate::login_config::with_read_lock(|c| c.resolve_default_model());
        }
        #[cfg(not(test))]
        (String::new(), String::new())
    }

    /// Look up a configured provider from the cached config.
    pub fn provider_config(&self, name: &str) -> Option<crate::config::ModelProvider> {
        if let Some(config) = self.config_cache.as_ref() {
            return config.model_providers.get(name).cloned();
        }
        #[cfg(test)]
        {
            return crate::login_config::get_provider_config(name).map(
                |(base_url, api_key, models)| crate::config::ModelProvider {
                    provider_type: None,
                    base_url,
                    api_key,
                    models,
                },
            );
        }
        #[cfg(not(test))]
        None
    }

    /// Fire-and-forget request to remove a provider via the ConfigActor.
    pub fn remove_provider(&self, name: &str) {
        self.send_config_msg(crate::actors::ConfigMsg::RemoveProvider {
            name: name.to_string(),
        });
        #[cfg(test)]
        {
            let _ = crate::login_config::remove_provider_config(name);
        }
    }

    /// Fire-and-forget request to update a provider's saved model list.
    pub fn set_provider_models(&self, name: &str, models: Vec<String>) {
        self.send_config_msg(crate::actors::ConfigMsg::SetProviderModels {
            name: name.to_string(),
            models: models.clone(),
        });
        #[cfg(test)]
        {
            if let Some((base_url, api_key, _)) = crate::login_config::get_provider_config(name) {
                let _ =
                    crate::login_config::save_provider_config(name, &base_url, &api_key, &models);
            }
        }
    }

    fn send_config_msg(&self, msg: crate::actors::ConfigMsg) {
        if let Some(ref tx) = self.config_tx {
            if tokio::runtime::Handle::try_current().is_ok() {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(msg).await;
                });
            }
        }
    }

    /// Record the height of the message viewport. Called by the render
    /// actor on each draw. Used by vim nav mode for element-level jumps.
    pub fn set_last_visible_height(&mut self, height: u16) {
        self.view.last_visible_height = height;
    }

    /// Record the width of the message content area. Called by the render
    /// actor on each draw. Used to keep core scroll math consistent with
    /// the actual wrapped Ratatui output.
    pub fn set_last_content_width(&mut self, width: u16) {
        self.view.last_content_width = width;
    }

    /// Record a model selection in recent history (max 5, no duplicates).
    pub fn record_model_usage(&mut self, provider: &str, model: &str) {
        let full = format!("{}/{}", provider, model);
        self.config.recent_models.retain(|m| m != &full);
        self.config.recent_models.push(full);
        if self.config.recent_models.len() > 5 {
            self.config.recent_models.remove(0);
        }
    }

    pub fn cache_generation(&self) -> u64 {
        self.view.message_gen
    }

    /// True when a provider and model are active/connected.
    pub fn has_models(&self) -> bool {
        !self.config.current_provider.is_empty() && !self.config.current_model.is_empty()
    }

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        crate::snapshot::visible_slice(&self.view.elements_cache, skip, take)
    }

    pub fn count(&self) -> usize {
        self.view.element_count.max(self.view.elements_cache.len())
    }

    pub fn element_count(&self) -> usize {
        self.view.element_count
    }

    pub fn total_lines(&self) -> usize {
        self.view.total_lines
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        crate::snapshot::scroll_offset(self.view.total_lines, self.view.scroll, visible_height)
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        crate::snapshot::scrollbar_metrics(self.view.total_lines, self.view.scroll, visible_height)
    }

    pub fn elements_cache(&self) -> &[Element] {
        self.view.elements_cache.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }

    /// Record that a command was invoked for palette ranking.
    pub fn record_command_usage(&mut self, name: &str) {
        let now = crate::update::now();
        let entry = self
            .config
            .command_usage
            .entry(name.to_string())
            .or_insert_with(|| crate::state::CommandUsage {
                count: 0,
                last_used: now,
            });
        entry.count += 1;
        entry.last_used = now;
    }

    /// Rank commands by fuzzy match score, recency boost, and usage count.
    /// Returns commands in ranked order, limited to `limit`.
    pub fn rank_commands(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<(&crate::commands::CommandDef, i32)> {
        let all: Vec<_> = self.registry.list();
        if query.is_empty() {
            rank_commands_empty_query(self, &all, limit)
        } else {
            rank_commands_with_query(self, query, &all, limit)
        }
    }

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
            crate::state::ModelSource::UserOverride,
        );
        self.config.theme_name = session.theme_name.clone();
        self.config.thinking_level = session.thinking_level;
        self.config.read_only = session.read_only;
        self.session.session_display_name =
            session.display_name.clone().or(Some(session.name.clone()));
        self.session.session_created_at = session.created_at;
        self.session.session_updated_at = session.updated_at;
        self.session.session_tree = session.session_tree.clone();
        self.messages_changed();
    }
}

fn rank_commands_empty_query<'a>(
    state: &'a AppState,
    all: &[&'a crate::commands::CommandDef],
    limit: usize,
) -> Vec<(&'a crate::commands::CommandDef, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .map(|cmd| {
            let usage = state.config.command_usage.get(&cmd.name);
            let score = compute_ranking_score("", cmd, usage);
            (*cmd, score)
        })
        .collect();
    ranked.sort_by_key(|(cmd, score)| (std::cmp::Reverse(*score), &cmd.category, &cmd.name));
    ranked.into_iter().take(limit).collect()
}

fn rank_commands_with_query<'a>(
    state: &'a AppState,
    query: &str,
    all: &[&'a crate::commands::CommandDef],
    limit: usize,
) -> Vec<(&'a crate::commands::CommandDef, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .filter_map(|cmd| {
            let base = crate::fuzzy::fuzzy_match(query, &cmd.name)
                .or_else(|| crate::fuzzy::fuzzy_match(query, &cmd.desc))?;
            let usage = state.config.command_usage.get(&cmd.name);
            let score = compute_ranking_score(query, cmd, usage) + base * 100;
            Some((*cmd, score))
        })
        .collect();
    ranked.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    ranked.into_iter().take(limit).collect()
}
fn has_provider_credentials(config: &crate::config::Config, provider: &str) -> bool {
    config
        .model_providers
        .get(provider)
        .map(|p| !p.api_key.is_empty())
        .unwrap_or(false)
}
