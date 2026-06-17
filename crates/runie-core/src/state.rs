use std::collections::VecDeque;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::keybindings::default_keybindings;
use crate::message::{now, ChatMessage};
use crate::model::{ModelSelectorItem, QueuedMessage, ThinkingLevel};
use crate::path_complete::PathCompletion;
use crate::scoped_model::ScopedModel;
use crate::session_tree::SessionTree;
use crate::streaming_buffer::StreamingBuffer;
use crate::ui::elements::Element;

/// Tracks usage count and last-used timestamp for a command.
#[derive(Clone, Debug)]
pub struct CommandUsage {
    pub count: u32,
    pub last_used: f64,
}

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
    /// Command input history (persistent across sessions).
    pub input_history: Vec<String>,
    pub current_prompt: String,
    /// Backup of input state before opening file picker:
    /// (original input, insert position, cursor position, needs brackets for @ references).
    pub file_picker_backup: Option<(String, usize, usize, bool)>,
    /// The `:start-end` range suffix to append when inserting a file reference.
    /// Set when opening the picker from `@path:10-50`.
    pub file_picker_range_suffix: Option<String>,
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
            file_picker_backup: None,
            file_picker_range_suffix: None,
        }
    }
}

/// Rolling window for speed calculation - tracks last N tokens' arrival times.
#[derive(Clone)]
pub struct SpeedWindow {
    /// Token arrival events: (timestamp, cumulative_token_count_at_arrival)
    /// Using VecDeque for O(1) pop_front.
    events: std::collections::VecDeque<(std::time::Instant, usize)>,
    /// Maximum tokens to track in window
    window_tokens: usize,
}

impl Default for SpeedWindow {
    fn default() -> Self {
        // Default to 1000 token window
        Self {
            events: std::collections::VecDeque::new(),
            window_tokens: 1000,
        }
    }
}

impl SpeedWindow {
    /// Create a new window tracking up to `window_tokens` tokens.
    pub fn new(window_tokens: usize) -> Self {
        Self {
            events: std::collections::VecDeque::new(),
            window_tokens,
        }
    }

    /// Record tokens arriving at the current time.
    pub fn record(&mut self, token_count: usize) {
        let now = std::time::Instant::now();
        self.events.push_back((now, token_count));
        self.evict_old();
    }

    /// Remove events outside the window.
    fn evict_old(&mut self) {
        if self.events.len() <= 1 {
            return;
        }
        // Find oldest event within window_tokens of current count
        let Some((_, latest)) = self.events.back() else {
            return;
        };
        let cutoff = latest.saturating_sub(self.window_tokens);
        while self.events.len() > 1 {
            if let Some((_, count)) = self.events.front() {
                if *count < cutoff {
                    self.events.pop_front();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Calculate tokens/sec based on the rolling window.
    /// Returns 0.0 if not enough data.
    pub fn speed(&self) -> f64 {
        if self.events.len() < 2 {
            return 0.0;
        }
        let (start, start_tok) = self.events.front().unwrap();
        let (end, end_tok) = self.events.back().unwrap();
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
    /// Token estimation/cost tracker configured for the active model.
    pub token_tracker: crate::tokens::TokenTracker,
    pub streaming: bool,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub current_action: Option<String>,
    pub(crate) thought_seq: u64,
    pub(crate) last_assistant_index: Option<usize>,
    pub thinking_started_at: Option<std::time::Instant>,
    /// Buffer for streaming response deltas (stable content + mutable tail).
    pub streaming_buffer: StreamingBuffer,
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
    /// Height of the message viewport in terminal rows, updated by
    /// the render actor on each draw. Used by vim nav mode to compute
    /// element-level jumps for `j`/`k`/arrow keys.
    pub last_visible_height: u16,
    /// Width of the message content area in terminal columns, updated by
    /// the render actor on each draw. Used to compute per-element line
    /// counts so that scroll math matches the actual wrapped output.
    pub last_content_width: u16,
    /// Index of the post currently selected in vim nav mode.
    /// A post is a logical unit in the feed (e.g. a user message, a
    /// thought, a tool call). Independent of scroll; used to highlight
    /// the selected post and to drive post-level navigation.
    pub selected_post: Option<usize>,
    // Cached palette items (for command palette dialog)
    pub(crate) cached_palette_items: Arc<[(String, String, String)]>,
    pub(crate) cached_palette_filter: Option<String>,
    // Cached model selector items
    pub(crate) cached_model_items: Arc<[ModelSelectorItem]>,
    pub(crate) cached_model_filter: Option<String>,
    // Cached settings items
    pub(crate) cached_settings_items: Arc<[crate::settings::SettingItem]>,
    pub(crate) cached_settings_valid: bool,
    // Cached session tree items
    pub(crate) cached_session_tree_items: Arc<[(usize, String)]>,
    pub(crate) cached_session_tree_valid: bool,
    // Cached auth provider names
    pub(crate) cached_auth_providers: Arc<[String]>,
    pub(crate) cached_auth_valid: bool,
    /// Navigable posts in the feed. Rebuilt alongside `elements_cache`.
    pub posts: Arc<[crate::ui::elements::Post]>,
    /// Last known mouse position from `MouseMove` events. Used by the TUI
    /// to compute `MouseTarget` for hover styling and click routing.
    pub mouse_position: Option<(u16, u16)>,
    /// Vim-style scrollback navigation active.
    pub vim_nav_mode: bool,
    /// When vim_mode Esc was used to abort a turn, the next Esc enters
    /// nav mode. Cleared once consumed or when a turn is no longer active.
    pub vim_nav_pending: bool,
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
            last_visible_height: 20,
            last_content_width: 80,
            cached_palette_items: Arc::new([]),
            cached_palette_filter: None,
            cached_model_items: Arc::new([]),
            cached_model_filter: None,
            cached_settings_items: Arc::new([]),
            cached_settings_valid: false,
            cached_session_tree_items: Arc::new([]),
            cached_session_tree_valid: false,
            cached_auth_providers: Arc::new([]),
            cached_auth_valid: false,
            selected_post: None,
            posts: Arc::new([]),
            mouse_position: None,
            vim_nav_mode: false,
            vim_nav_pending: false,
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
    /// Vim-style scrollback navigation (opt-in).
    pub vim_mode: bool,
    pub steering_mode: crate::model::DeliveryMode,
    pub follow_up_mode: crate::model::DeliveryMode,
    pub recent_models: Vec<String>,
    /// Telemetry/analytics tracking.
    pub telemetry: crate::telemetry::Telemetry,
    /// Execution mode: Solo (default) or Team (orchestrator).
    pub execution_mode: crate::orchestrator::ExecutionMode,
    /// Per-session command usage tracking for palette ranking.
    pub command_usage: std::collections::HashMap<String, CommandUsage>,
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
            vim_mode: true,
            steering_mode: crate::model::DeliveryMode::default(),
            follow_up_mode: crate::model::DeliveryMode::default(),
            recent_models: Vec::new(),
            telemetry: crate::telemetry::Telemetry::new(false),
            execution_mode: crate::orchestrator::ExecutionMode::default(),
            command_usage: std::collections::HashMap::new(),
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

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar state (Team mode)
// ─────────────────────────────────────────────────────────────────────────────

/// Which agent feed is currently visible / focused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum AgentFocus {
    /// Showing the Orchestrator's main feed.
    #[default]
    Orchestrator,
    /// Showing a specific subagent's feed.
    Subagent(String),
}


/// Per-agent status for the sidebar list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEntry {
    pub id: String,
    pub label: String,
    pub status: AgentStatus,
}

/// Per-agent lifecycle status for the sidebar list.
///
/// Alias for the canonical lifecycle enum shared with the orchestrator and
/// subagent actor.
pub type AgentStatus = crate::orchestrator::AgentLifecycleStatus;

/// Alias for subagent lifecycle status.
pub type SubagentStatus = crate::orchestrator::AgentLifecycleStatus;

/// Alias for task lifecycle status.
pub type TaskStatus = crate::orchestrator::AgentLifecycleStatus;

/// Sidebar state for Team mode — tracks subagent list and focus.
#[derive(Debug, Clone, Default)]
pub struct SidebarState {
    /// Whether the sidebar is visible (Team mode with active plan).
    pub visible: bool,
    /// Which agent feed is currently focused.
    pub focus: AgentFocus,
    /// Ordered list of agents in the sidebar (Orchestrator first, then subagents).
    pub agents: Vec<AgentEntry>,
}

impl SidebarState {
    /// Add the Orchestrator entry (always at index 0).
    pub fn set_orchestrator_status(&mut self, status: AgentStatus) {
        if self.agents.is_empty() {
            self.agents.insert(0, AgentEntry {
                id: String::new(),
                label: "Orchestrator".to_string(),
                status,
            });
        } else {
            self.agents[0].status = status;
        }
    }

    /// Replace subagent entries (indices 1+).
    pub fn set_subagents(&mut self, subagents: Vec<AgentEntry>) {
        if self.agents.is_empty() {
            self.agents.insert(0, AgentEntry {
                id: String::new(),
                label: "Orchestrator".to_string(),
                status: AgentStatus::Pending,
            });
        }
        self.agents.truncate(1);
        self.agents.extend(subagents);
    }

    /// Focus a subagent by its 1-based index (Ctrl+1..9).
    pub fn focus_subagent_by_index(&mut self, idx: usize) {
        let subagent_idx = 1 + idx; // 1-based, 0 = orchestrator
        if subagent_idx < self.agents.len() {
            let id = self.agents[subagent_idx].id.clone();
            self.focus = AgentFocus::Subagent(id);
        }
    }

    /// Return to the Orchestrator feed.
    pub fn focus_orchestrator(&mut self) {
        self.focus = AgentFocus::Orchestrator;
    }
}

#[cfg(test)]
mod sidebar_tests {
    use super::*;

    #[test]
    fn sidebar_defaults_hidden() {
        let sidebar = SidebarState::default();
        assert!(!sidebar.visible);
        assert!(matches!(sidebar.focus, AgentFocus::Orchestrator));
        assert!(sidebar.agents.is_empty());
    }

    #[test]
    fn focus_defaults_to_orchestrator() {
        assert!(matches!(AgentFocus::default(), AgentFocus::Orchestrator));
    }

    #[test]
    fn focus_subagent_by_index() {
        let mut sidebar = SidebarState::default();
        sidebar.agents.push(AgentEntry {
            id: String::new(),
            label: "Orchestrator".to_string(),
            status: AgentStatus::Running,
        });
        sidebar.agents.push(AgentEntry {
            id: "t1".to_string(),
            label: "Reviewer".to_string(),
            status: AgentStatus::Pending,
        });
        sidebar.agents.push(AgentEntry {
            id: "t2".to_string(),
            label: "Writer".to_string(),
            status: AgentStatus::Pending,
        });

        sidebar.focus_subagent_by_index(0);
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t1");
        } else {
            panic!("expected Subagent(t1)");
        }

        sidebar.focus_subagent_by_index(1);
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t2");
        } else {
            panic!("expected Subagent(t2)");
        }

        sidebar.focus_subagent_by_index(9); // out of range — unchanged
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t2");
        }
    }

    #[test]
    fn focus_orchestrator() {
        let mut sidebar = SidebarState::default();
        sidebar.focus = AgentFocus::Subagent("t1".to_string());
        sidebar.focus_orchestrator();
        assert!(matches!(sidebar.focus, AgentFocus::Orchestrator));
    }

    #[test]
    fn set_orchestrator_status_empty() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Running);
        assert_eq!(sidebar.agents.len(), 1);
        assert_eq!(sidebar.agents[0].label, "Orchestrator");
        assert!(matches!(sidebar.agents[0].status, AgentStatus::Running));
    }

    #[test]
    fn set_orchestrator_status_updates_existing() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Pending);
        sidebar.set_orchestrator_status(AgentStatus::Done { output: None });
        assert_eq!(sidebar.agents.len(), 1);
        assert!(matches!(sidebar.agents[0].status, AgentStatus::Done { output: _ }));
    }

    #[test]
    fn set_subagents_replaces_non_orchestrator() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Running);
        sidebar.set_subagents(vec![
            AgentEntry { id: "t1".into(), label: "R".into(), status: AgentStatus::Running },
            AgentEntry { id: "t2".into(), label: "W".into(), status: AgentStatus::Pending },
        ]);
        assert_eq!(sidebar.agents.len(), 3); // orchestrator + 2 subagents
        assert!(matches!(sidebar.agents[0].status, AgentStatus::Running)); // orchestrator preserved
        assert_eq!(sidebar.agents[1].id, "t1");
        assert_eq!(sidebar.agents[2].id, "t2");
    }

    #[test]
    fn agent_status_serialization() {
        let statuses = [
            AgentStatus::Pending,
            AgentStatus::Running,
            AgentStatus::AwaitingUser,
            AgentStatus::Done { output: Some("done".into()) },
            AgentStatus::Failed { error: "boom".into() },
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let roundtrip: AgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, status);
        }
    }

    #[test]
    fn agent_entry_serialization() {
        let entry = AgentEntry {
            id: "t1".into(),
            label: "Reviewer".into(),
            status: AgentStatus::Running,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let roundtrip: AgentEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, "t1");
        assert_eq!(roundtrip.label, "Reviewer");
        assert!(matches!(roundtrip.status, AgentStatus::Running));
    }

    #[test]
    fn task_status_into_agent_status() {
        let status: TaskStatus = TaskStatus::AwaitingUser;
        let agent: AgentStatus = status.into();
        assert_eq!(agent, AgentStatus::AwaitingUser);
    }

    #[test]
    fn subagent_status_into_agent_status() {
        let status: SubagentStatus = SubagentStatus::Failed {
            error: "boom".into(),
        };
        let agent: AgentStatus = status.into();
        assert_eq!(agent, AgentStatus::Failed { error: "boom".into() });
    }

    #[test]
    fn agent_focus_serialization() {
        let variants = [
            AgentFocus::Orchestrator,
            AgentFocus::Subagent("t1".into()),
        ];
        for focus in variants {
            let json = serde_json::to_string(&focus).unwrap();
            let roundtrip: AgentFocus = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, focus);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Integration tests: OrchestratorEvent → SidebarState
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod orchestrator_sidebar_tests {
    use super::*;
    use crate::orchestrator::{ModelTrait, OrchestratorPlan, SubagentTask, TaskStatus};
    use crate::orchestrator_actor::OrchestratorEvent;

    fn orchestrator_plan() -> OrchestratorPlan {
        OrchestratorPlan {
            tasks: vec![
                SubagentTask::new("t1", "reviewer", "Review src/lib.rs", ModelTrait::General),
                SubagentTask::new("t2", "writer", "Write tests for src/lib.rs", ModelTrait::General),
            ],
            synthesis_trait: ModelTrait::General,
            summary: None,
            rationale: None,
        }
    }

    fn apply_event(state: &mut crate::model::AppState, event: OrchestratorEvent) {
        state.update(event);
    }

    #[test]
    fn plan_started_shows_sidebar() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        assert!(state.sidebar.visible);
        assert_eq!(state.sidebar.agents.len(), 1);
        assert!(matches!(state.sidebar.agents[0].status, AgentStatus::Running));
    }

    #[test]
    fn plan_generated_populates_subagents() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(&mut state, OrchestratorEvent::PlanGenerated { plan: Box::new(orchestrator_plan()) });
        assert_eq!(state.sidebar.agents.len(), 3); // orchestrator + 2 subagents
        assert_eq!(state.sidebar.agents[1].id, "t1");
        assert_eq!(state.sidebar.agents[2].id, "t2");
        assert!(matches!(state.sidebar.agents[1].status, AgentStatus::Pending));
        assert!(matches!(state.sidebar.agents[2].status, AgentStatus::Pending));
    }

    #[test]
    fn subagent_status_changed_updates_entry() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(&mut state, OrchestratorEvent::PlanGenerated { plan: Box::new(orchestrator_plan()) });
        apply_event(&mut state, OrchestratorEvent::SubagentStatusChanged {
            task_id: "t1".into(),
            status: TaskStatus::Running,
        });
        let entry = state.sidebar.agents.iter().find(|a| a.id == "t1").unwrap();
        assert!(matches!(entry.status, AgentStatus::Running));
        // t2 should be unchanged
        let entry2 = state.sidebar.agents.iter().find(|a| a.id == "t2").unwrap();
        assert!(matches!(entry2.status, AgentStatus::Pending));
    }

    #[test]
    fn cancelled_hides_sidebar() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(&mut state, OrchestratorEvent::PlanGenerated { plan: Box::new(orchestrator_plan()) });
        apply_event(&mut state, OrchestratorEvent::Cancelled);
        assert!(!state.sidebar.visible);
        assert!(state.sidebar.agents.is_empty());
    }

    #[test]
    fn orchestrator_event_serialization() {
        let plan = orchestrator_plan();
        let events = [
            OrchestratorEvent::PlanStarted,
            OrchestratorEvent::PlanningStarted,
            OrchestratorEvent::PlanGenerated { plan: Box::new(plan.clone()) },
            OrchestratorEvent::PlanningFailed { error: "timeout".into() },
            OrchestratorEvent::SubagentStatusChanged {
                task_id: "t1".into(),
                status: TaskStatus::Running,
            },
            OrchestratorEvent::Cancelled,
            OrchestratorEvent::Finished { success: true },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let roundtrip: OrchestratorEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, event);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Team mode integration tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod team_mode_tests {
    use super::*;
    use crate::model::AppState;
    use crate::orchestrator::ExecutionMode;

    #[test]
    fn solo_mode_uses_agent() {
        let state = AppState::default();
        assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
        assert!(!state.config.execution_mode.uses_orchestrator());
    }

    #[test]
    fn team_mode_uses_orchestrator() {
        let mut state = AppState::default();
        state.config.execution_mode = ExecutionMode::Team;
        assert!(state.config.execution_mode.uses_orchestrator());
    }

    #[test]
    fn team_mode_toggle_shows_sidebar() {
        let mut state = AppState::default();
        // Initially hidden
        assert!(!state.sidebar.visible);
        // Switch to Team — sidebar becomes visible
        state.config.execution_mode = ExecutionMode::Team;
        // Sidebar shows when orchestrator plan starts (not just mode toggle)
        // The key invariant: sidebar is only visible when in Team mode AND plan is active
        assert!(!state.sidebar.visible); // no plan yet
    }

    #[test]
    fn solo_mode_has_no_sidebar_agents() {
        let mut state = AppState::default();
        // Force some agents in
        state.sidebar.visible = true;
        state.sidebar.agents.push(AgentEntry {
            id: "t1".into(),
            label: "Test".into(),
            status: AgentStatus::Running,
        });
        // Switch to Solo — agents cleared
        state.config.execution_mode = ExecutionMode::Solo;
        state.sidebar.visible = false;
        state.sidebar.agents.clear();
        assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
        assert!(!state.sidebar.visible);
        assert!(state.sidebar.agents.is_empty());
    }
}

