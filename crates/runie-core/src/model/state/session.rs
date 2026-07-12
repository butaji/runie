use serde::{Deserialize, Serialize};

use crate::edit_preview::EditPreview;
use crate::keybindings::default_keybindings;
use crate::message::{now, ChatMessage};
use crate::model::ThinkingLevel;
use crate::path_complete::PathCompletion;
use crate::scoped_model::ScopedModel;
use crate::session::tree::SessionTree;

use super::CommandUsage;

/// Session state — messages, tree, pending edits.
/// Fields are public for test setup; production code should use accessors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub messages: Vec<ChatMessage>,
    #[serde(skip)]
    pub session_tree: Option<SessionTree>,
    pub session_display_name: Option<String>,
    pub session_created_at: f64,
    pub session_updated_at: f64,
    pub pending_edits: Vec<EditPreview>,
    pub image_attachments: Vec<String>,
}

impl PartialEq for SessionState {
    fn eq(&self, other: &Self) -> bool {
        self.messages == other.messages
            && self.session_display_name == other.session_display_name
            && self.session_created_at == other.session_created_at
            && self.session_updated_at == other.session_updated_at
            && self.pending_edits == other.pending_edits
            && self.image_attachments == other.image_attachments
    }
}

impl Default for SessionState {
    fn default() -> Self {
        let t = now();
        Self {
            messages: Vec::new(),
            session_tree: None,
            session_display_name: None,
            session_created_at: t,
            session_updated_at: t,
            pending_edits: Vec::new(),
            image_attachments: Vec::new(),
        }
    }
}

impl SessionState {
    /// Immutable access to messages.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Mutable access to messages.
    pub fn messages_mut(&mut self) -> &mut Vec<ChatMessage> {
        &mut self.messages
    }

    /// Immutable access to session display name.
    pub fn session_display_name(&self) -> Option<&str> {
        self.session_display_name.as_deref()
    }

    /// Mutable access to session display name.
    pub fn session_display_name_mut(&mut self) -> &mut Option<String> {
        &mut self.session_display_name
    }

    /// Session creation timestamp.
    pub fn session_created_at(&self) -> f64 {
        self.session_created_at
    }

    /// Mutable access to session creation timestamp.
    pub fn session_created_at_mut(&mut self) -> &mut f64 {
        &mut self.session_created_at
    }

    /// Mutable access to session updated timestamp.
    pub fn session_updated_at_mut(&mut self) -> &mut f64 {
        &mut self.session_updated_at
    }
}

/// Why the active provider/model is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSource {
    /// Loaded from config defaults (explicit provider/model or first configured).
    ConfigDefault,
    /// Explicitly chosen by the user this session; /reload should not revert it.
    UserOverride,
}

/// Configuration state — provider, model, theme, keybindings.
/// Fields are public for test setup; production code should use accessors.
#[derive(Clone)]
pub struct ConfigState {
    /// Default provider (from `[provider]` or first configured).
    pub provider: Option<String>,
    /// Default model (from `[model]`).
    pub default_model: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    pub keybindings: std::collections::HashMap<String, String>,
    pub theme_name: String,
    pub thinking_level: ThinkingLevel,
    pub read_only: bool,
    pub scoped_models: Vec<ScopedModel>,
    pub scoped_index: usize,
    /// Truncation limits for tool output. Loaded from `[truncation]` in
    /// `config.toml`. See `runie-agent::truncate::TruncationPolicy`.
    pub truncation: crate::config::TruncationSection,
    /// Vim-style scrollback navigation (opt-in).
    pub vim_mode: bool,
    pub steering_mode: crate::model::DeliveryMode,
    pub follow_up_mode: crate::model::DeliveryMode,
    pub recent_models: Vec<String>,
    /// Telemetry settings. Loaded from `[telemetry]` in `config.toml`.
    pub telemetry: crate::config::TelemetrySection,
    /// Per-session command usage tracking for palette ranking.
    pub command_usage: std::collections::HashMap<String, CommandUsage>,
    /// Why the current provider/model is active.
    pub model_source: ModelSource,
    /// Provider configurations loaded from config.toml.
    /// Previously accessed via AppState.config_cache.
    pub model_providers: std::collections::HashMap<String, crate::config::ModelProvider>,
}

impl Default for ConfigState {
    fn default() -> Self {
        // In production (no RUNIE_MOCK), the app starts with no provider.
        // The startup hook detects this and auto-opens the login dialog
        // so the user is immediately productive. In dev (RUNIE_MOCK=1),
        // the mock provider is the default so the app works out of the box.
        // In unit tests we also default to a connected model so the bulk of
        // the rendering suite does not need to set one manually.
        #[cfg(test)]
        let (provider, model) = ("mock".to_string(), "echo".to_string());
        #[cfg(not(test))]
        let (provider, model) = if crate::provider::is_mock_enabled() {
            ("mock".to_owned(), crate::provider::mock_model())
        } else {
            (String::new(), String::new())
        };
        Self {
            provider: None,
            default_model: None,
            current_provider: provider,
            current_model: model,
            keybindings: default_keybindings(),
            theme_name: crate::theme_detection::detect_system_appearance()
                .default_theme_name()
                .into(),
            thinking_level: ThinkingLevel::Off,
            read_only: false,
            scoped_models: Vec::new(),
            scoped_index: 0,
            truncation: crate::config::TruncationSection::default(),
            vim_mode: true,
            steering_mode: crate::model::DeliveryMode::default(),
            follow_up_mode: crate::model::DeliveryMode::default(),
            recent_models: Vec::new(),
            telemetry: crate::config::TelemetrySection::default(),
            command_usage: std::collections::HashMap::new(),
            model_source: ModelSource::ConfigDefault,
            model_providers: std::collections::HashMap::new(),
        }
    }
}

impl ConfigState {
    /// Immutable access to keybindings.
    pub fn keybindings(&self) -> &std::collections::HashMap<String, String> {
        &self.keybindings
    }

    /// Mutable access to keybindings.
    pub fn keybindings_mut(&mut self) -> &mut std::collections::HashMap<String, String> {
        &mut self.keybindings
    }

    /// Current provider name.
    pub fn current_provider(&self) -> &str {
        &self.current_provider
    }

    /// Mutable access to current provider.
    pub fn current_provider_mut(&mut self) -> &mut String {
        &mut self.current_provider
    }

    /// Current model name.
    pub fn current_model(&self) -> &str {
        &self.current_model
    }

    /// Mutable access to current model.
    pub fn current_model_mut(&mut self) -> &mut String {
        &mut self.current_model
    }

    /// Whether read-only mode is active.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// Mutable access to read_only flag.
    pub fn read_only_mut(&mut self) -> &mut bool {
        &mut self.read_only
    }

    /// Current model source (why this provider/model is active).
    pub fn model_source(&self) -> ModelSource {
        self.model_source
    }

    /// Mutable access to model source.
    pub fn model_source_mut(&mut self) -> &mut ModelSource {
        &mut self.model_source
    }

    /// Vim mode enabled.
    pub fn vim_mode(&self) -> bool {
        self.vim_mode
    }

    /// Mutable access to vim_mode.
    pub fn vim_mode_mut(&mut self) -> &mut bool {
        &mut self.vim_mode
    }

    /// Mutable access to scoped_models.
    pub fn scoped_models_mut(&mut self) -> &mut Vec<ScopedModel> {
        &mut self.scoped_models
    }

    /// Mutable access to thinking_level.
    pub fn thinking_level_mut(&mut self) -> &mut ThinkingLevel {
        &mut self.thinking_level
    }

    /// Mutable access to truncation.
    pub fn truncation_mut(&mut self) -> &mut crate::config::TruncationSection {
        &mut self.truncation
    }

    /// Immutable access to model_providers.
    pub fn model_providers(
        &self,
    ) -> &std::collections::HashMap<String, crate::config::ModelProvider> {
        &self.model_providers
    }

    /// Mutable access to model_providers.
    pub fn model_providers_mut(
        &mut self,
    ) -> &mut std::collections::HashMap<String, crate::config::ModelProvider> {
        &mut self.model_providers
    }

    /// Whether telemetry is enabled (mirrors `TelemetrySection.enabled`).
    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry.enabled
    }

    /// Mutable access to telemetry enabled flag.
    pub fn telemetry_enabled_mut(&mut self) -> &mut bool {
        &mut self.telemetry.enabled
    }
}

/// Completion/suggestion state — path and @ mentions.
/// Fields are public for test setup.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompletionState {
    pub path_suggestions: Option<Vec<PathCompletion>>,
    pub path_selected: Option<usize>,
    pub at_suggestions: Option<Vec<String>>,
    pub at_selected: Option<usize>,
    pub last_at_query: Option<String>,
}

impl CompletionState {
    /// Mutable access to path_suggestions.
    pub fn path_suggestions_mut(&mut self) -> &mut Option<Vec<PathCompletion>> {
        &mut self.path_suggestions
    }
}
