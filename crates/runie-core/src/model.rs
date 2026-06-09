//! Model — Application State (mutable borrow, no cloning per event)
use crate::snapshot::Snapshot;
use crate::ui::elements::Element;
pub use crate::message::{ChatMessage, Role, now};

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum DeliveryMode {
    /// Each message triggers a separate LLM call
    #[default]
    OneAtATime,
    /// All queued messages delivered together in one LLM call
    All,
}

/// Thinking level for reasoning-intensive tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThinkingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Low => "\nThink briefly before responding.",
            Self::Medium => "\nThink step by step before responding.",
            Self::High => "\nThink deeply and thoroughly. Consider edge cases and alternatives.",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl std::str::FromStr for ThinkingLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off), "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium), "high" => Ok(Self::High),
            _ => Err(format!("Unknown thinking level: {s}")),
        }
    }
}
pub use crate::scoped_model::ScopedModel;

pub use crate::model_catalog::{ModelInfo, model_catalog, filter_models, build_model_selector_items};
#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    /// Cursor position in input (0 = before first char)
    pub cursor_pos: usize,
    pub streaming: bool,
    pub scroll: usize,
    pub thinking_started_at: Option<std::time::Instant>,
    pub request_queue: std::collections::VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    /// How steering messages are delivered to the LLM
    pub steering_mode: DeliveryMode,
    /// How follow-up messages are delivered to the LLM
    pub follow_up_mode: DeliveryMode,
    pub next_id: u64,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub intermediate_step_count: usize,
    pub animation_frame: u32,
    pub turn_active: bool,
    pub current_action: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    /// Active theme name (resolved by runie-tui)
    pub theme_name: String,
    /// Command registry for slash command dispatch
    pub registry: crate::commands::CommandRegistry,
    /// Set to true when the user requests quit
    pub should_quit: bool,
    /// Currently open dialog (if any)
    pub open_dialog: Option<crate::commands::DialogState>,
    /// Default provider from config (for /new reset)
    pub config_provider: String,
    /// Default model from config (for /new reset)
    pub config_model: String,
    /// Current keybindings (reloadable)
    pub keybindings: std::collections::HashMap<String, String>,
    /// Optional display name for the current session
    pub session_display_name: Option<String>,
    /// Session creation timestamp (unix seconds)
    pub session_created_at: f64,
    /// Session last-updated timestamp (unix seconds)
    pub session_updated_at: f64,
    /// Current thinking level (off → low → medium → high)
    pub thinking_level: ThinkingLevel,
    /// Read-only mode — when true, only safe tools are exposed to the LLM
    pub read_only: bool,
    /// Scoped models for Ctrl+P cycling (defaults to first 10 from catalog)
    pub scoped_models: Vec<ScopedModel>,
    /// Current index in scoped_models cycling
    pub scoped_index: usize,
    /// Recently used models (last 5) for the model selector dialog
    pub recent_models: Vec<String>,
    /// Pending file edits awaiting user approval
    pub pending_edits: Vec<crate::edit_preview::EditPreview>,
    /// Loaded skills from ~/.runie/skills/ and ./.runie/skills/
    pub skills: Vec<crate::skills::Skill>,
    /// Opt-in telemetry collector
    pub telemetry: crate::telemetry::Telemetry,
    /// Loaded prompt templates
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    /// Active prompt template name (empty = default)
    pub current_prompt: String,
    /// Base64 image attachments pending in the input field
    pub image_attachments: Vec<String>,
    /// Session tree for branching conversation history
    pub session_tree: Option<crate::session_tree::SessionTree>,

    /// Number of commands sent to agent but not yet completed
    pub inflight: usize,
    /// @-ref file lookup suggestions
    pub at_suggestions: Option<Vec<String>>,
    /// Selected index in @-ref suggestions
    pub at_selected: Option<usize>,
    /// Last @-ref query to avoid redundant filesystem calls
    pub last_at_query: Option<String>,
    /// Path completion suggestions (Tab-triggered)
    pub path_suggestions: Option<Vec<crate::path_complete::PathCompletion>>,
    /// Selected index in path completion suggestions
    pub path_selected: Option<usize>,
    /// Global collapse flag — when true, ALL thoughts/tools render collapsed.
    /// New elements automatically respect this setting.
    pub all_collapsed: bool,
    /// Cached index of last assistant message — O(1) lookup for append_response
    pub(crate) last_assistant_index: Option<usize>,
    pub(crate) thought_seq: u64,
    pub(crate) input_history: Vec<String>,
    pub(crate) history_pos: Option<usize>,
    pub(crate) undo_stack: Vec<(String, usize)>,
    pub(crate) redo_stack: Vec<(String, usize)>,
    pub input_flash: u8,
    pub placeholder: String,
    element_count: usize,
    elements_cache: Vec<Element>,
    line_counts: Vec<usize>,
    total_lines: usize,
    dirty: bool,
    message_gen: u64,
    cached_gen: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(), input: String::new(), cursor_pos: 0,
            streaming: false, scroll: 0, thinking_started_at: None,
            request_queue: std::collections::VecDeque::new(),
            message_queue: Vec::new(),
            steering_mode: DeliveryMode::OneAtATime,
            follow_up_mode: DeliveryMode::OneAtATime,
            next_id: 0, current_request_id: None, turn_started_at: None,
            current_tool_name: None, tool_started_at: None,
            intermediate_step_count: 0, animation_frame: 0,
            turn_active: false, current_action: None,
            current_provider: "mock".into(), current_model: "echo".into(),
            theme_name: "silkcircuit-neon".into(),
            registry: crate::commands::CommandRegistry::new(),
            should_quit: false, open_dialog: None,
            config_provider: "mock".into(), config_model: "echo".into(),
            keybindings: crate::keybindings::default_keybindings(),
            session_display_name: None,
            session_created_at: now(), session_updated_at: now(),
            thinking_level: ThinkingLevel::Off, read_only: false,
            scoped_models: Vec::new(), scoped_index: 0,
            recent_models: Vec::new(), pending_edits: Vec::new(),
            skills: Vec::new(),
            telemetry: crate::telemetry::Telemetry::new(false),
            prompts: Vec::new(), current_prompt: String::new(),
            image_attachments: Vec::new(),
            session_tree: None,
            inflight: 0,
            at_suggestions: None, at_selected: None, last_at_query: None,
            path_suggestions: None, path_selected: None,
            all_collapsed: false, last_assistant_index: None, thought_seq: 0,
            input_history: Vec::new(), history_pos: None,
            undo_stack: Vec::new(), redo_stack: Vec::new(),
            input_flash: 0, placeholder: "Type a message to start...".into(),
            element_count: 0, elements_cache: Vec::new(),
            line_counts: Vec::new(), total_lines: 0,
            dirty: true, message_gen: 1, cached_gen: 0,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.turn_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.tool_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        SPINNER_CHARS[(self.animation_frame % SPINNER_FRAMES) as usize]
    }

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn messages_changed(&mut self) {
        self.message_gen = self.message_gen.wrapping_add(1);
        self.session_updated_at = now();
        self.dirty = true;
    }

    fn palette_items(&self) -> Vec<(String, String, String)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::CommandPalette { filter, .. }) => filter.clone(),
            _ => return Vec::new(),
        };
        let mut items: Vec<_> = crate::commands::filter_commands(&self.registry, &filter)
            .into_iter()
            .map(|cmd| (cmd.name.clone(), cmd.description.clone(), cmd.category.as_str().to_string()))
            .collect();
        let f = filter.to_lowercase();
        for skill in &self.skills {
            if skill.user_invocable
                && (f.is_empty()
                    || skill.name.to_lowercase().contains(&f)
                    || skill.description.to_lowercase().contains(&f))
            {
                items.push((skill.name.clone(), skill.description.clone(), "Skill".to_string()));
            }
        }
        items
    }

    fn session_tree_items(&self) -> Vec<(usize, String)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::SessionTree { filter, .. }) => *filter,
            _ => return Vec::new(),
        };
        match self.session_tree.as_ref() {
            Some(tree) => tree.filtered_walk(filter)
                .into_iter()
                .map(|(depth, node)| {
                    let preview = format!("[{}] {}", node.message.role.as_str(), node.message.content.chars().take(60).collect::<String>());
                    (depth, preview)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn model_selector_items(&self) -> Vec<(String, String, String, bool, bool)> {
        let (filter, _) = match &self.open_dialog {
            Some(crate::commands::DialogState::ModelSelector { filter, .. }) => (filter.clone(), 0),
            _ => return Vec::new(),
        };
        build_model_selector_items(
            &model_catalog(),
            &self.recent_models,
            &filter,
            &self.current_provider,
            &self.current_model,
        )
    }

    /// Record a model selection in recent history (max 5, no duplicates).
    pub fn record_model_usage(&mut self, provider: &str, model: &str) {
        let full = format!("{}/{}", provider, model);
        self.recent_models.retain(|m| m != &full);
        self.recent_models.push(full);
        if self.recent_models.len() > 5 {
            self.recent_models.remove(0);
        }
    }

    pub fn cache_generation(&self) -> u64 {
        self.message_gen
    }

    /// Rebuild cache only when messages changed — O(n) but gated
    pub fn ensure_fresh(&mut self) {
        if self.dirty && self.message_gen != self.cached_gen {
            self.elements_cache = crate::ui::LazyCache::rebuild(self);
            self.element_count = self.elements_cache.len();
            self.line_counts = self.elements_cache.iter().map(|e| e.line_count()).collect();
            self.total_lines = self.line_counts.iter().sum();
            self.cached_gen = self.message_gen;
        }
        self.dirty = false;
    }

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.elements_cache.is_empty() {
            return &[];
        }
        let start = skip.min(self.element_count).min(self.elements_cache.len());
        let end = (start + take).min(self.element_count).min(self.elements_cache.len());
        &self.elements_cache[start..end]
    }

    pub fn count(&self) -> usize {
        self.element_count.max(self.elements_cache.len())
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn elements_cache(&self) -> &[Element] {
        &self.elements_cache
    }

    pub(crate) fn line_counts(&self) -> &[usize] {
        &self.line_counts
    }

    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.turn_active {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            changed = true;
        }
        if self.input_flash > 0 {
            self.input_flash -= 1;
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
    }

    /// Build an immutable Snapshot for the render actor.
    /// The event loop calls this after ensure_fresh(); the render
    /// actor receives it via channel and draws without touching state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            elements: self.elements_cache.clone(),
            line_counts: self.line_counts.clone(),
            total_lines: self.total_lines,
            input: self.input.clone(),
            cursor_pos: self.cursor_pos,
            hint_text: self.hint_text(),
            at_suggestions: self.at_suggestions.clone(),
            at_selected: self.at_selected,
            path_suggestions: self.path_suggestions.clone(),
            path_selected: self.path_selected,
            turn_active: self.turn_active,
            input_flash: self.input_flash,
            placeholder: self.placeholder.clone(),
            spinner_frame: self.spinner_frame(),
            scroll: self.scroll,
            turn_elapsed_secs: self.turn_elapsed_secs(),
            provider: self.current_provider.clone(),
            model: self.current_model.clone(),
            theme_name: self.theme_name.clone(),
            thinking_level: self.thinking_level,
            read_only: self.read_only,
            queue_count: self.message_queue.len() + self.request_queue.len(),
            dialog: self.open_dialog.clone(),
            palette_items: self.palette_items(),
            model_selector_items: self.model_selector_items(),
            pending_edits: self.pending_edits.clone(),
            scoped_models: self.scoped_models.clone(),
            settings_items: crate::update::settings_dialog::build_setting_items(self),
            session_tree_items: self.session_tree_items(),
            image_attachments: self.image_attachments.clone(),
            auth_providers: {
                let auth = crate::auth::AuthStorage::load();
                auth.tokens.keys().cloned().collect()
            },
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn total_tokens(&self) -> usize {
        self.messages.iter().map(|m| crate::tokens::estimate_tokens(&m.content)).sum()
    }

    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.messages.iter().enumerate().rev() {
            accumulated += crate::tokens::estimate_tokens(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.messages.len() {
            match self.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }
        let removed_count = cut_idx;
        self.messages.drain(..cut_idx);
        let summary = format!("[Compacted: {} earlier messages removed, keeping ~{} tokens]", removed_count, keep_recent_tokens);
        self.messages.insert(0, ChatMessage {
            role: Role::System,
            content: summary.clone(),
            timestamp: now(),
            id: "compaction".to_string(),
            ..Default::default()
        });
        self.messages_changed();
        summary
    }
}


