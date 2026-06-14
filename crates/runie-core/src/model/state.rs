//! Core application state types and simple accessors.

use crate::ui::elements::Element;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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
    /// All thinking levels in cycle order (low → high).
    /// Single source of truth for UI selectors.
    pub const ALL: &'static [ThinkingLevel] = &[
        ThinkingLevel::Off,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ];

    /// All thinking levels in cycle order. See [`ALL`](Self::ALL).
    pub fn all() -> &'static [ThinkingLevel] {
        Self::ALL
    }

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

    /// Returns the I Ching hexagram for this thinking level.
    /// Maps to 3-bit representation: 000=earth, 111=heaven.
    pub fn hexagram(&self) -> &'static str {
        match self {
            Self::Off => "☷",    // 000 - earth (no thinking)
            Self::Low => "☵",    // 010 - water (minimal thinking)
            Self::Medium => "☳", // 100 - thunder (moderate thinking)
            Self::High => "☰",   // 111 - heaven (deep thinking)
        }
    }
}

impl std::str::FromStr for ThinkingLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("Unknown thinking level: {s}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

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
    /// Command input history (persistent across sessions)
    pub input_history: Vec<String>,
    /// True while the user is in vim feed-navigation mode (j/k/g/G etc.).
    /// Only meaningful when `config.vim_mode` is enabled.
    pub vim_nav_mode: bool,
    /// When vim_mode Esc was used to abort a turn, the next Esc enters
    /// nav mode. Cleared once consumed or when a turn is no longer active.
    pub vim_nav_pending: bool,
    /// Backup of input state before opening file picker:
    /// (original input, insert position, cursor position, needs brackets for @ references).
    pub file_picker_backup: Option<(String, usize, usize, bool)>,
    pub pending_agent_edit: Option<crate::agent_profiles::AgentProfile>,
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
            input_history: Vec::new(),
            vim_nav_mode: false,
            vim_nav_pending: false,
            file_picker_backup: None,
            pending_agent_edit: None,
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

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.view.elements_cache.is_empty() {
            return &[];
        }
        let start = skip
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
        let end = (start + take)
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
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

    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }
}
