//! Model — Application State (mutable borrow, no cloning per event)
use std::sync::Arc;
use crate::snapshot::Snapshot;
use crate::ui::elements::Element;
pub use crate::message::{ChatMessage, Role, now};

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

/// Approximate token count from text (4 chars ≈ 1 token).
pub fn count_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

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
    pub session: crate::state::SessionState,
    pub input: crate::state::InputState,
    pub agent: crate::state::AgentState,
    pub view: crate::state::ViewState,
    pub config: crate::state::ConfigState,
    pub completion: crate::state::CompletionState,

    pub streaming: bool,
    pub thinking_started_at: Option<std::time::Instant>,
    pub steering_mode: DeliveryMode,
    pub follow_up_mode: DeliveryMode,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub animation_frame: u32,
    pub current_action: Option<String>,
    pub registry: crate::commands::CommandRegistry,
    pub should_quit: bool,
    pub open_dialog: Option<crate::commands::DialogState>,
    pub recent_models: Vec<String>,
    pub pending_edits: Vec<crate::edit_preview::EditPreview>,
    pub skills: Vec<crate::skills::Skill>,
    pub telemetry: crate::telemetry::Telemetry,
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    pub current_prompt: String,
    pub image_attachments: Vec<String>,
    pub all_collapsed: bool,
    pub(crate) last_assistant_index: Option<usize>,
    pub(crate) thought_seq: u64,
    pub(crate) input_history: Vec<String>,
    pub transient_message: Option<String>,
    pub transient_until: Option<std::time::Instant>,
    pub transient_level: Option<crate::event::TransientLevel>,
    cached_palette_items: Vec<(String, String, String)>,
    cached_palette_filter: Option<String>,
    cached_model_items: Vec<(String, String, String, bool, bool)>,
    cached_model_filter: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: crate::state::SessionState::default(),
            input: crate::state::InputState::default(),
            agent: crate::state::AgentState::default(),
            view: crate::state::ViewState::default(),
            config: crate::state::ConfigState::default(),
            completion: crate::state::CompletionState::default(),
            streaming: false,
            thinking_started_at: None,
            steering_mode: DeliveryMode::OneAtATime,
            follow_up_mode: DeliveryMode::OneAtATime,
            next_id: 0,
            intermediate_step_count: 0,
            animation_frame: 0,
            current_action: None,
            registry: crate::commands::CommandRegistry::new(),
            should_quit: false,
            open_dialog: None,
            recent_models: Vec::new(),
            pending_edits: Vec::new(),
            skills: Vec::new(),
            telemetry: crate::telemetry::Telemetry::new(false),
            prompts: Vec::new(),
            current_prompt: String::new(),
            image_attachments: Vec::new(),
            all_collapsed: false,
            last_assistant_index: None,
            thought_seq: 0,
            input_history: Vec::new(),
            transient_message: None,
            transient_until: None,
            transient_level: None,
            cached_palette_items: Vec::new(),
            cached_palette_filter: None,
            cached_model_items: Vec::new(),
            cached_model_filter: None,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.agent.turn_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.agent.tool_started_at.map(|t| t.elapsed().as_secs_f64())
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
        self.view.dirty = true;
    }

    pub fn messages_changed(&mut self) {
        self.view.message_gen = self.view.message_gen.wrapping_add(1);
        self.session.session_updated_at = now();
        self.view.dirty = true;
    }

    fn palette_items(&mut self) -> Vec<(String, String, String)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::CommandPalette { filter, .. }) => filter.clone(),
            _ => {
                self.cached_palette_filter = None;
                self.cached_palette_items.clear();
                return Vec::new();
            }
        };
        if Some(&filter) != self.cached_palette_filter.as_ref() {
            self.cached_palette_filter = Some(filter.clone());
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
            self.cached_palette_items = items;
        }
        self.cached_palette_items.clone()
    }

    fn session_tree_items(&self) -> Vec<(usize, String)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::SessionTree { filter, .. }) => *filter,
            _ => return Vec::new(),
        };
        match self.session.session_tree.as_ref() {
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

    fn model_selector_items(&mut self) -> Vec<(String, String, String, bool, bool)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::ModelSelector { filter, .. }) => filter.clone(),
            _ => {
                self.cached_model_filter = None;
                self.cached_model_items.clear();
                return Vec::new();
            }
        };
        if Some(&filter) != self.cached_model_filter.as_ref() {
            self.cached_model_filter = Some(filter.clone());
            self.cached_model_items = build_model_selector_items(
                &model_catalog(),
                &self.recent_models,
                &filter,
                &self.config.current_provider,
                &self.config.current_model,
            );
        }
        self.cached_model_items.clone()
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
        self.view.message_gen
    }

    /// Rebuild cache only when messages changed — O(n) but gated
    pub fn ensure_fresh(&mut self) {
        if self.view.dirty && self.view.message_gen != self.view.cached_gen {
            let elements = crate::ui::LazyCache::rebuild(self);
            self.view.element_count = elements.len();
            let line_counts: Vec<usize> = elements.iter().map(|e| e.line_count()).collect();
            self.view.total_lines = line_counts.iter().sum();
            self.view.line_counts = line_counts.into();
            self.view.elements_cache = elements.into();
            self.view.cached_gen = self.view.message_gen;
        }
        self.view.dirty = false;
    }

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.view.elements_cache.is_empty() {
            return &[];
        }
        let start = skip.min(self.view.element_count).min(self.view.elements_cache.len());
        let end = (start + take).min(self.view.element_count).min(self.view.elements_cache.len());
        &self.view.elements_cache[start..end]
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

    pub fn elements_cache(&self) -> &[Element] {
        self.view.elements_cache.as_ref()
    }

    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.agent.turn_active {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            self.update_speed();
            changed = true;
        }
        if self.input.input_flash > 0 {
            self.input.input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        // Animate token counters toward their target values
        if self.animate_tokens() {
            changed = true;
        }
        if changed {
            self.view.dirty = true;
        }
    }

    /// Animate token display values toward their actual values.
    /// Returns true if the display values changed.
    fn animate_tokens(&mut self) -> bool {
        // Track changes in actual values
        if self.agent.tokens_in != self.agent.tokens_in_prev {
            self.agent.tokens_in_prev = self.agent.tokens_in;
        }
        if self.agent.tokens_out != self.agent.tokens_out_prev {
            self.agent.tokens_out_prev = self.agent.tokens_out;
        }
        // Ease-out interpolation: 15% of remaining per tick
        let t_in = self.agent.tokens_in as f64;
        let t_out = self.agent.tokens_out as f64;
        let d_in = t_in - self.agent.tokens_in_display;
        let d_out = t_out - self.agent.tokens_out_display;
        let c1 = if d_in.abs() < 0.5 {
            let n = self.agent.tokens_in_display.round() as usize != t_in as usize;
            if n { self.agent.tokens_in_display = t_in; }
            n
        } else {
            self.agent.tokens_in_display += d_in * 0.15;
            true
        };
        let c2 = if d_out.abs() < 0.5 {
            let n = self.agent.tokens_out_display.round() as usize != t_out as usize;
            if n { self.agent.tokens_out_display = t_out; }
            n
        } else {
            self.agent.tokens_out_display += d_out * 0.15;
            true
        };
        c1 || c2
    }

    /// Update streaming speed using rolling window of last 1000 tokens.
    /// Called every animation tick (~200ms).
    pub fn update_speed(&mut self) {
        let now = std::time::Instant::now();
        let last = self.agent.last_speed_update.get_or_insert(now);
        let elapsed = now.duration_since(*last).as_secs_f64();

        if elapsed < 0.05 {
            return; // Too soon, wait for next tick
        }

        let prev_tokens = self.agent.tokens_at_last_speed;
        let delta_tokens = self.agent.tokens_out.saturating_sub(prev_tokens);

        if delta_tokens > 0 {
            // Record new tokens in rolling window
            self.agent.speed_window.record(self.agent.tokens_out);
            self.agent.tokens_at_last_speed = self.agent.tokens_out;
            // Calculate speed from rolling window
            self.agent.speed_tps = self.agent.speed_window.speed();
            *last = now;
        } else if elapsed > 1.0 {
            // No new tokens for 1s+ — decay speed toward 0
            self.agent.speed_tps *= 0.5;
            if self.agent.speed_tps < 0.1 {
                self.agent.speed_tps = 0.0;
            }
        }
    }

    fn clear_expired_transient(&mut self) -> bool {
        if let Some(until) = self.transient_until {
            if std::time::Instant::now() > until {
                self.transient_message = None;
                self.transient_until = None;
                self.transient_level = None;
                return true;
            }
        }
        false
    }

    /// Build an immutable Snapshot for the render actor.
    /// The event loop calls this after ensure_fresh(); the render
    /// actor receives it via channel and draws without touching state.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            elements: Arc::clone(&self.view.elements_cache),
            line_counts: Arc::clone(&self.view.line_counts),
            total_lines: self.view.total_lines,
            input: self.input.input.clone(),
            cursor_pos: self.input.cursor_pos,
            hint_text: self.hint_text(),
            path_suggestions: self.completion.path_suggestions.clone(),
            path_selected: self.completion.path_selected,
            turn_active: self.agent.turn_active,
            input_flash: self.input.input_flash,
            placeholder: self.input.placeholder.clone(),
            ghost_completion: self.input.ghost_completion.clone(),
            spinner_frame: self.spinner_frame(),
            scroll: self.view.scroll,
            turn_elapsed_secs: self.turn_elapsed_secs(),
            provider: self.config.current_provider.clone(),
            model: self.config.current_model.clone(),
            theme_name: self.config.theme_name.clone(),
            thinking_level: self.config.thinking_level,
            read_only: self.config.read_only,
            queue_count: self.agent.message_queue.len() + self.agent.request_queue.len(),
            dialog: self.open_dialog.clone(),
            palette_items: self.palette_items(),
            model_selector_items: self.model_selector_items(),
            pending_edits: self.pending_edits.clone(),
            scoped_models: self.config.scoped_models.clone(),
            settings_items: crate::update::settings_dialog::build_setting_items(self),
            session_tree_items: self.session_tree_items(),
            image_attachments: self.image_attachments.clone(),
            auth_providers: crate::auth::AuthStorage::load().tokens.keys().cloned().collect(),
            transient_message: self.transient_message.clone(),
            transient_level: self.transient_level,
            tokens_in: self.agent.tokens_in,
            tokens_out: self.agent.tokens_out,
            speed_tps: self.agent.speed_tps,
            tokens_in_display: self.agent.tokens_in_display,
            tokens_out_display: self.agent.tokens_out_display,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }

    pub fn total_tokens(&self) -> usize {
        self.session.messages.iter().map(|m| crate::tokens::estimate_tokens(&m.content)).sum()
    }

    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.session.messages.iter().enumerate().rev() {
            accumulated += crate::tokens::estimate_tokens(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.session.messages.len() {
            match self.session.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }
        let removed_count = cut_idx;
        self.session.messages.drain(..cut_idx);
        let summary = format!("[Compacted: {} earlier messages removed, keeping ~{} tokens]", removed_count, keep_recent_tokens);
        self.session.messages.insert(0, ChatMessage {
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


