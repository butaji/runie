use crate::edit_preview::EditPreview;
use crate::keybindings::default_keybindings;
use crate::message::{now, ChatMessage};
use crate::model::ThinkingLevel;
use crate::path_complete::PathCompletion;
use crate::scoped_model::ScopedModel;
use crate::session_tree::SessionTree;

use super::CommandUsage;

#[derive(Clone)]
pub struct SessionState {
    pub messages: Vec<ChatMessage>,
    pub session_tree: Option<SessionTree>,
    pub session_display_name: Option<String>,
    pub session_created_at: f64,
    pub session_updated_at: f64,
    pub pending_edits: Vec<EditPreview>,
    pub image_attachments: Vec<String>,
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

/// Why the active provider/model is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSource {
    /// Loaded from config defaults (explicit provider/model or first configured).
    ConfigDefault,
    /// Explicitly chosen by the user this session; /reload should not revert it.
    UserOverride,
}

#[derive(Clone)]
pub struct ConfigState {
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
    /// Telemetry/analytics tracking.
    pub telemetry: crate::telemetry::Telemetry,
    /// Per-session command usage tracking for palette ranking.
    pub command_usage: std::collections::HashMap<String, CommandUsage>,
    /// Why the current provider/model is active.
    pub model_source: ModelSource,
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
        let (provider, model) = if crate::provider_registry::is_mock_enabled() {
            ("mock".to_string(), "echo".to_string())
        } else {
            (String::new(), String::new())
        };
        Self {
            current_provider: provider,
            current_model: model,
            keybindings: default_keybindings(),
            theme_name: "runie".into(),
            thinking_level: ThinkingLevel::Off,
            read_only: false,
            scoped_models: Vec::new(),
            scoped_index: 0,
            truncation: crate::config::TruncationSection::default(),
            vim_mode: true,
            steering_mode: crate::model::DeliveryMode::default(),
            follow_up_mode: crate::model::DeliveryMode::default(),
            recent_models: Vec::new(),
            telemetry: crate::telemetry::Telemetry::new(false),
            command_usage: std::collections::HashMap::new(),
            model_source: ModelSource::ConfigDefault,
        }
    }
}

#[derive(Clone, Default)]
pub struct CompletionState {
    pub path_suggestions: Option<Vec<PathCompletion>>,
    pub path_selected: Option<usize>,
    pub at_suggestions: Option<Vec<String>>,
    pub at_selected: Option<usize>,
    pub last_at_query: Option<String>,
}
