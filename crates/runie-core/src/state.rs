use std::collections::VecDeque;
use std::sync::Arc;

use crate::keybindings::default_keybindings;
use crate::message::{now, ChatMessage};
use crate::model::{QueuedMessage, ThinkingLevel};
use crate::path_complete::PathCompletion;
use crate::scoped_model::ScopedModel;
use crate::session_tree::SessionTree;
use crate::ui::elements::Element;

#[derive(Clone)]
pub struct InputState {
    pub input: String,
    pub cursor_pos: usize,
    pub(crate) undo_stack: Vec<(String, usize)>,
    pub(crate) redo_stack: Vec<(String, usize)>,
    pub(crate) history_pos: Option<usize>,
    pub input_flash: u8,
    pub placeholder: String,
    /// Ghost completion suffix shown in gray after the cursor.
    pub ghost_completion: Option<String>,
    /// Tab-completion state stored as raw fields (avoid circular dep).
    pub tab_complete_prefix: Option<String>,
    pub tab_complete_matches: Vec<String>,
    pub tab_complete_index: usize,
    /// Top visible line index for multi-line input scrolling.
    pub input_scroll: usize,
    // Fields moved from AppState (Phase 1: add without removing outer fields)
    pub(crate) input_history: Vec<String>,
    pub current_prompt: String,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_pos: None,
            input_flash: 0,
            placeholder: "Type a message to start...".into(),
            ghost_completion: None,
            tab_complete_prefix: None,
            tab_complete_matches: Vec::new(),
            tab_complete_index: 0,
            input_scroll: 0,
            input_history: Vec::new(),
            current_prompt: String::new(),
        }
    }
}

/// Rolling window for speed calculation - tracks last N tokens' arrival times.
#[derive(Clone)]
pub struct SpeedWindow {
    /// Token arrival events: (timestamp, cumulative_token_count_at_arrival)
    events: Vec<(std::time::Instant, usize)>,
    /// Maximum tokens to track in window
    window_tokens: usize,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        // Default to 1000 token window
        Self {
            events: Vec::new(),
            window_tokens: 1000,
        }
    }
}

impl SpeedWindow {
    /// Create a new window tracking up to `window_tokens` tokens.
    pub fn new(window_tokens: usize) -> Self {
        Self {
            events: Vec::new(),
            window_tokens,
        }
    }

    /// Record tokens arriving at the current time.
    pub fn record(&mut self, token_count: usize) {
        let now = std::time::Instant::now();
        self.events.push((now, token_count));
        self.evict_old();
    }

    /// Remove events outside the window.
    fn evict_old(&mut self) {
        if self.events.len() <= 1 {
            return;
        }
        // Find oldest event within window_tokens of current count
        let Some((_, latest)) = self.events.last() else {
            return;
        };
        let cutoff = latest.saturating_sub(self.window_tokens);
        while self.events.len() > 1 && self.events[0].1 < cutoff {
            self.events.remove(0);
        }
    }

    /// Calculate tokens/sec based on the rolling window.
    /// Returns 0.0 if not enough data.
    pub fn speed(&self) -> f64 {
        if self.events.len() < 2 {
            return 0.0;
        }
        let (start, start_tok) = &self.events[0];
        let (end, end_tok) = self.events.last().unwrap();
        if start_tok == end_tok {
            return 0.0;
        }
        let elapsed = end.duration_since(*start).as_secs_f64();
        if elapsed < 0.001 {
            return 0.0;
        }
        (end_tok - start_tok) as f64 / elapsed
    }

    /// Clear the window.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Number of events in window.
    pub fn len(&self) -> usize {
        self.events.len()
    }
    /// True if window is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Clone, Default)]
pub struct AgentState {
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    /// Cumulative input tokens sent to LLM (all turns).
    pub tokens_in: usize,
    /// Cumulative output tokens received from LLM (all turns).
    pub tokens_out: usize,
    /// Output tokens in the current turn (for speed calculation).
    pub turn_tokens_out: usize,
    /// Current streaming speed in tokens/sec (rolling window).
    pub speed_tps: f64,
    /// Rolling window for speed calculation.
    pub speed_window: SpeedWindow,
    /// Last time speed was updated.
    pub last_speed_update: Option<std::time::Instant>,
    /// Token count snapshot at last speed update.
    pub tokens_at_last_speed: usize,
    /// Animated display value for tokens_in (smooth interpolation).
    pub tokens_in_display: f64,
    /// Animated display value for tokens_out (smooth interpolation).
    pub tokens_out_display: f64,
    /// Previous token_in value for detecting changes.
    pub tokens_in_prev: usize,
    /// Previous token_out value for detecting changes.
    pub tokens_out_prev: usize,
    // Fields moved from AppState (Phase 1: add without removing outer fields)
    pub streaming: bool,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub current_action: Option<String>,
    pub(crate) thought_seq: u64,
    pub(crate) last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,
}

#[derive(Clone)]
pub struct ViewState {
    pub scroll: usize,
    pub elements_cache: Arc<[Element]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
    pub dirty: bool,
    pub cached_gen: u64,
    pub message_gen: u64,
    pub element_count: usize,
    // Animation/scroll state
    pub animation_frame: u32,
    pub all_collapsed: bool,
    // Cached palette items (for command palette dialog)
    pub(crate) cached_palette_items: Vec<(String, String, String)>,
    pub(crate) cached_palette_filter: Option<String>,
    // Cached model selector items
    pub(crate) cached_model_items: Vec<(String, String, String, bool, bool)>,
    pub(crate) cached_model_filter: Option<String>,
}

impl ViewState {
    pub fn elements_cache(&self) -> &[Element] {
        self.elements_cache.as_ref()
    }

    pub fn line_counts(&self) -> &[usize] {
        self.line_counts.as_ref()
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scroll: 0,
            elements_cache: Arc::new([]),
            line_counts: Arc::new([]),
            total_lines: 0,
            dirty: true,
            cached_gen: 0,
            message_gen: 1,
            element_count: 0,
            animation_frame: 0,
            all_collapsed: false,
            cached_palette_items: Vec::new(),
            cached_palette_filter: None,
            cached_model_items: Vec::new(),
            cached_model_filter: None,
        }
    }
}

#[derive(Clone)]
pub struct SessionState {
    pub messages: Vec<ChatMessage>,
    pub session_tree: Option<SessionTree>,
    pub session_display_name: Option<String>,
    pub session_created_at: f64,
    pub session_updated_at: f64,
    // Fields moved from AppState (Phase 1: add without removing outer fields)
    pub pending_edits: Vec<crate::edit_preview::EditPreview>,
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

#[derive(Clone)]
pub struct ConfigState {
    pub current_provider: String,
    pub current_model: String,
    pub config_provider: String,
    pub config_model: String,
    pub keybindings: std::collections::HashMap<String, String>,
    pub theme_name: String,
    pub thinking_level: ThinkingLevel,
    pub read_only: bool,
    pub scoped_models: Vec<ScopedModel>,
    pub scoped_index: usize,
    /// Truncation limits for tool output. Loaded from `[truncation]` in
    /// `config.toml`. See `runie-agent::truncate::TruncationPolicy`.
    pub truncation: crate::config_reload::TruncationSection,
    // Fields moved from AppState (Phase 1: add without removing outer fields)
    pub steering_mode: crate::model::DeliveryMode,
    pub follow_up_mode: crate::model::DeliveryMode,
    pub recent_models: Vec<String>,
    /// Telemetry/analytics tracking.
    pub telemetry: crate::telemetry::Telemetry,
}

impl Default for ConfigState {
    fn default() -> Self {
        // In production (no RUNIE_MOCK), the app starts with no provider.
        // The startup hook detects this and auto-opens the login dialog
        // so the user is immediately productive. In dev (RUNIE_MOCK=1),
        // the mock provider is the default so the app works out of the box.
        let (provider, model) = if crate::provider_registry::is_mock_enabled() {
            ("mock".to_string(), "echo".to_string())
        } else {
            (String::new(), String::new())
        };
        Self {
            current_provider: provider.clone(),
            current_model: model.clone(),
            config_provider: provider,
            config_model: model,
            keybindings: default_keybindings(),
            theme_name: "runie".into(),
            thinking_level: ThinkingLevel::Off,
            read_only: false,
            scoped_models: Vec::new(),
            scoped_index: 0,
            truncation: crate::config_reload::TruncationSection::default(),
            steering_mode: crate::model::DeliveryMode::default(),
            follow_up_mode: crate::model::DeliveryMode::default(),
            recent_models: Vec::new(),
            telemetry: crate::telemetry::Telemetry::new(false),
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
