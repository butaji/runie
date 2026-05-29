//! AppState and related types.
//!
//! Phase 2 of architecture migration: decompose monolithic AppState into focused sub-states.

use crate::components::{DiffViewer, PaletteCommand, ModelPicker};
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use crate::components::PermissionAction;
pub use crate::components::onboarding::{Onboarding, OnboardingStep};
pub use runie_ai::model_fetcher::ModelInfo;
use runie_ai::TokenUsage;
use runie_core::SlashCommand;
use crossterm::event::KeyEvent;

// ─── Sub-state modules ─────────────────────────────────────────────────────────

pub mod agent;
pub mod chat;
pub mod layout;
pub mod mode;
pub mod overlay;
pub mod system;

// Re-export sub-state types
pub use agent::AgentState;
pub use chat::ChatState;
pub use layout::LayoutState;
pub use mode::UiModeState;
pub use overlay::OverlayState;
pub use system::SystemState;

// ─── Shared Types (used by sub-states) ────────────────────────────────────────

/// P1-REMAINING-1 FIX: Track Ctrl+C double-tap to prevent accidental text loss
#[derive(Clone)]
pub struct ClearInputConfirm {
    pub pending: bool,
    pub last_press: Option<std::time::Instant>,
}

impl Default for ClearInputConfirm {
    fn default() -> Self {
        Self {
            pending: false,
            last_press: None,
        }
    }
}

impl ClearInputConfirm {
    /// Check if the user wants to clear input (requires double-tap within 2 seconds)
    pub fn wants_clear(&mut self) -> bool {
        let now = std::time::Instant::now();
        const CLEAR_CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

        if self.pending {
            // Second tap - clear confirmed
            if let Some(last) = self.last_press {
                if now.duration_since(last) < CLEAR_CONFIRM_TIMEOUT {
                    self.pending = false;
                    self.last_press = None;
                    return true;
                }
            }
            // Timeout expired, reset
            self.pending = false;
        }

        // First tap - request confirmation
        self.pending = true;
        self.last_press = Some(now);
        false
    }

    /// Check if there's a pending clear request (for showing hint)
    pub fn is_pending(&self) -> bool {
        self.pending
    }
}

#[derive(Clone)]
pub struct AnimationState {
    pub braille_frame: usize,
    pub rewind_braille_frame: usize,
    pub streaming_cursor_visible: bool,
    pub interrupt_fade_start: Option<std::time::Instant>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            braille_frame: 0,
            rewind_braille_frame: 0,
            streaming_cursor_visible: true,
            interrupt_fade_start: None,
        }
    }
}

#[derive(Clone)]
pub struct TopBarState {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub agent_count: Option<usize>,
    pub context_badges: Vec<String>,
    pub context_pct: Option<f32>,
    pub context_bar_pct: Option<f32>,
    pub context_window: Option<usize>,
    pub estimated_tokens: Option<usize>,
}

impl Default for TopBarState {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            checks_passed: None,
            checks_total: None,
            percentage: None,
            agent_count: None,
            context_badges: Vec::new(),
            context_pct: None,
            context_bar_pct: None,
            context_window: Some(128_000),
            estimated_tokens: Some(0),
        }
    }
}

/// Pending permission request (queued when in blocking mode)
#[derive(Clone, Debug)]
pub struct PendingPermission {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_args: String,
}

#[derive(Clone)]
pub struct PermissionModalState {
    pub tool: Option<String>,
    pub args: Option<String>,
    pub desc: Option<String>,
    pub tool_call_id: Option<String>,
    // P0-1 FIX: Track timeout for permission modal
    pub timeout_start: Option<std::time::Instant>,
    pub timed_out: bool,
    // BG-1 FIX: Queue for pending permission requests
    pub pending_queue: Vec<PendingPermission>,
    // P2-6 FIX: Progressive disclosure - show advanced options toggle
    pub show_advanced: bool,
}

impl Default for PermissionModalState {
    fn default() -> Self {
        Self {
            tool: None,
            args: None,
            desc: None,
            tool_call_id: None,
            timeout_start: None,
            timed_out: false,
            pending_queue: Vec::new(),
            show_advanced: false,
        }
    }
}

#[derive(Clone)]
pub struct CommandPaletteState {
    pub open: bool,
    pub filter: String,
    pub selected: usize,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            filter: String::new(),
            selected: 0,
        }
    }
}

#[derive(Clone)]
pub struct ScrollState {
    pub feed_offset: usize,
    pub diff_offset: usize,
    pub tree_offset: usize,
    /// True if user has manually scrolled up (auto-scroll should pause)
    pub user_scrolled_up: bool,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            feed_offset: 0,
            diff_offset: 0,
            tree_offset: 0,
            user_scrolled_up: false,
        }
    }
}

// ─── AppState (using sub-states) ──────────────────────────────────────────────

/// AppState is the main application state, decomposed into focused sub-states.
/// 
/// For backward compatibility, fields are kept at the top level AND organized into sub-states.
/// This allows both `state.messages` (backward compat) and `state.chat.messages` (new API).
#[derive(Clone)]
pub struct AppState {
    // Flat fields for backward compatibility (mirror sub-state contents)
    pub messages: Vec<crate::components::MessageItem>,
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub input_right_info: String,
    pub scroll: ScrollState,
    pub input_history: Vec<String>,
    pub input_history_index: Option<usize>,
    pub input_draft: String,

    pub agent_running: bool,
    pub current_model: Option<String>,
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub agent_start_time: Option<std::time::Instant>,
    pub background_jobs: Vec<crate::components::status_bar::BackgroundJob>,
    pub thinking_start: Option<std::time::Instant>,
    pub thinking_duration: Option<std::time::Duration>,
    pub is_thinking: bool,

    pub show_sidebar: bool,
    pub terminal_size: (u16, u16),
    pub top_bar: TopBarState,

    pub running: bool,
    pub mock_mode: bool,
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<std::time::Instant>,
    pub clear_input_confirm: ClearInputConfirm,

    pub permission_modal: PermissionModalState,
    pub command_palette: CommandPaletteState,
    pub model_picker: Option<ModelPicker>,
    pub diff_viewer: Option<DiffViewer>,
    pub session_tree: crate::components::SessionTreeNavigator,

    pub animation: AnimationState,
    pub onboarding: Option<Onboarding>,
    pub mode: TuiMode,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            // Chat
            messages: Vec::new(),
            textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(),
            scroll: ScrollState::default(),
            input_history: Vec::new(),
            input_history_index: None,
            input_draft: String::new(),
            // Agent
            agent_running: false,
            current_model: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            agent_start_time: None,
            background_jobs: Vec::new(),
            thinking_start: None,
            thinking_duration: None,
            is_thinking: false,
            // Layout
            show_sidebar: false,
            terminal_size: (0, 0),
            top_bar: TopBarState::default(),
            // System
            running: true,
            mock_mode: false,
            status_header: None,
            status_details: None,
            status_start_time: None,
            clear_input_confirm: ClearInputConfirm::default(),
            // Overlay
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            model_picker: None,
            diff_viewer: None,
            session_tree: crate::components::SessionTreeNavigator::new(),
            // Other
            animation: AnimationState::default(),
            onboarding: None,
            mode: TuiMode::Chat,
        }
    }
}

// ─── Standalone Widget Render Functions ────────────────────────────────────────

/// Render top bar from state (repo/branch/path info)

#[derive(Debug, Clone, PartialEq)]
pub enum TuiMode {
    Chat,
    Overlay,
    Select,
    Permission,
    CommandPalette,
    DiffViewer,
    SessionTree,
    Onboarding,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Input (TextArea handles most keys internally)
    // These variants are no longer used since textarea.input() is called directly
    // Kept for compatibility but they do nothing
    Submit,
    TextareaKey(KeyEvent),
    InsertNewline,

    // App
    Quit,
    ToggleSidebar,
    OpenCommandPalette,
    CloseModal,
    ConfirmModal,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,

    // Permission
    PermissionConfirm,
    PermissionCancel,
    PermissionAlways,
    PermissionSkip,

    // Command palette
    CommandPaletteFilter(char),
    CommandPaletteBackspace,
    CommandPaletteUp,
    CommandPaletteDown,
    CommandPaletteConfirm,
    // P1-1 FIX: Cancel argument mode in command palette
    CommandPaletteCancelArgument,

    // Events from outside
    AgentEvent(AgentEvent),

    // Animation
    Tick,
    CursorBlink,

    // Slash commands
    SlashCommand(runie_core::slash_command::SlashCommand),

    // Session tree
    ToggleSessionTree,
    SessionTreeUp,
    SessionTreeDown,
    SessionTreeConfirm,

    // Onboarding
    OnboardingNext,
    OnboardingBack,
    OnboardingNavigateUp,
    OnboardingNavigateDown,
    OnboardingSelectProvider(usize),
    OnboardingSelectModel(usize),
    OnboardingKeyInput(char),
    OnboardingKeyBackspace,
    OnboardingSearchInput(char),
    OnboardingSearchBackspace,
    OnboardingSubmit,
    OnboardingSkip,

    // P0-1 FIX: Permission timeout
    PermissionTimeout,

    // Select/Overlay navigation (model picker)
    SelectUp,
    SelectDown,
    SelectConfirm,
    SelectToggleDetails,

    // Input
    ClearInput,
    // P1-REMAINING-1 FIX: ClearInputConfirm - requires double-tap to clear text
    ClearInputConfirm,
    ClearChat,
    DirectCommand(PaletteCommand),
    Paste(String),

    // Model fetching
    ModelsFetched(Vec<ModelInfo>),
    ModelsFetchFailed(String),

    // Terminal
    Resize(u16, u16),

    // P0-1 FIX: Stop — fired by Ctrl+C signal handler to interrupt agent
    Stop,

    // Model picker shortcut
    SwitchModel,

    // State initialization (fixes direct mutations in tui_run.rs)
    SetGitInfo { repo: String, branch: String, path: String },
    SetTopBarMockChecks {
        checks_passed: Option<usize>,
        checks_total: Option<usize>,
        percentage: Option<f32>,
        context_badges: Vec<String>,
    },
    SetTopBarRealChecks { context_badges: Vec<String> },
    SetInputRightInfo(String),
    EnterOnboarding,

    // Critical #3, #4: State mutations that were direct in tui_run.rs
    SetCurrentModel(Option<String>),
    SetMockMode(bool),
    ResetAgentState,
    UpdateTopBarContext { model: String, context_window: Option<usize>, estimated_tokens: Option<usize> },

    // Input history navigation
    HistoryUp,
    HistoryDown,

    // Copy last response to clipboard
    CopyLastResponse,
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        use Msg::*;
        match (self, other) {
            (Submit, Submit) => true,
            (TextareaKey(a), TextareaKey(b)) => a == b,
            (InsertNewline, InsertNewline) => true,
            (Quit, Quit) => true,
            (ToggleSidebar, ToggleSidebar) => true,
            (OpenCommandPalette, OpenCommandPalette) => true,
            (CloseModal, CloseModal) => true,
            (ConfirmModal, ConfirmModal) => true,
            (ScrollUp, ScrollUp) => true,
            (ScrollDown, ScrollDown) => true,
            (ScrollPageUp, ScrollPageUp) => true,
            (ScrollPageDown, ScrollPageDown) => true,
            (PermissionConfirm, PermissionConfirm) => true,
            (PermissionCancel, PermissionCancel) => true,
            (PermissionAlways, PermissionAlways) => true,
            (PermissionSkip, PermissionSkip) => true,
            (CommandPaletteFilter(a), CommandPaletteFilter(b)) => a == b,
            (CommandPaletteBackspace, CommandPaletteBackspace) => true,
            (CommandPaletteUp, CommandPaletteUp) => true,
            (CommandPaletteDown, CommandPaletteDown) => true,
            (CommandPaletteConfirm, CommandPaletteConfirm) => true,
            (CommandPaletteCancelArgument, CommandPaletteCancelArgument) => true,
            (AgentEvent(_), AgentEvent(_)) => true,
            (Tick, Tick) => true,
            (CursorBlink, CursorBlink) => true,
            (SlashCommand(_), SlashCommand(_)) => true,
            (ToggleSessionTree, ToggleSessionTree) => true,
            (SessionTreeUp, SessionTreeUp) => true,
            (SessionTreeDown, SessionTreeDown) => true,
            (SessionTreeConfirm, SessionTreeConfirm) => true,
            (OnboardingNext, OnboardingNext) => true,
            (OnboardingBack, OnboardingBack) => true,
            (OnboardingNavigateUp, OnboardingNavigateUp) => true,
            (OnboardingNavigateDown, OnboardingNavigateDown) => true,
            (OnboardingSelectProvider(a), OnboardingSelectProvider(b)) => a == b,
            (OnboardingSelectModel(a), OnboardingSelectModel(b)) => a == b,
            (OnboardingKeyInput(a), OnboardingKeyInput(b)) => a == b,
            (OnboardingKeyBackspace, OnboardingKeyBackspace) => true,
            (OnboardingSearchInput(a), OnboardingSearchInput(b)) => a == b,
            (OnboardingSearchBackspace, OnboardingSearchBackspace) => true,
            (OnboardingSubmit, OnboardingSubmit) => true,
            (OnboardingSkip, OnboardingSkip) => true,
            (ClearInput, ClearInput) => true,
            (ClearInputConfirm, ClearInputConfirm) => true,
            (ClearChat, ClearChat) => true,
            (DirectCommand(a), DirectCommand(b)) => a == b,
            (Paste(a), Paste(b)) => a == b,
            (ModelsFetched(a), ModelsFetched(b)) => a == b,
            (ModelsFetchFailed(a), ModelsFetchFailed(b)) => a == b,
            (Resize(a_w, a_h), Resize(b_w, b_h)) => a_w == b_w && a_h == b_h,
            (Stop, Stop) => true,
            (PermissionTimeout, PermissionTimeout) => true,
            (SelectUp, SelectUp) => true,
            (SelectDown, SelectDown) => true,
            (SelectConfirm, SelectConfirm) => true,
            (SelectToggleDetails, SelectToggleDetails) => true,
            (SwitchModel, SwitchModel) => true,
            (SetGitInfo { .. }, SetGitInfo { .. }) => true,
            (SetTopBarMockChecks { .. }, SetTopBarMockChecks { .. }) => true,
            (SetTopBarRealChecks { .. }, SetTopBarRealChecks { .. }) => true,
            (SetInputRightInfo(a), SetInputRightInfo(b)) => a == b,
            (EnterOnboarding, EnterOnboarding) => true,
            (SetCurrentModel(a), SetCurrentModel(b)) => a == b,
            (SetMockMode(a), SetMockMode(b)) => a == b,
            (ResetAgentState, ResetAgentState) => true,
            (UpdateTopBarContext { .. }, UpdateTopBarContext { .. }) => true,
            (HistoryUp, HistoryUp) => true,
            (HistoryDown, HistoryDown) => true,
            (CopyLastResponse, CopyLastResponse) => true,
            _ => false,
        }
    }
}

// ─── Cmd ────────────────────────────────────────────────────────────────────────
// Effects returned by update() to be executed by the runtime

#[derive(Debug, Clone)]
pub enum Cmd {
    SpawnAgent { messages: Vec<AgentMessage> },
    SendPermission { decision: PermissionDecision },
    SlashCommand(SlashCommand),
    SaveSettings { provider: String, model: String, api_key: String },
    FetchModels { provider_id: String, api_key: String },
    // P1-4 FIX: Rollback — reverts partial tool changes on permission cancel
    Rollback { tool_call_id: String },
    // P0-1 FIX: Interrupt — cancels the running agent task
    Interrupt,
}

impl PartialEq for Cmd {
    fn eq(&self, other: &Self) -> bool {
        use Cmd::*;
        match (self, other) {
            (SpawnAgent { .. }, SpawnAgent { .. }) => true, // Can't compare messages
            (SendPermission { decision: a }, SendPermission { decision: b }) => a == b,
            (SlashCommand(_), SlashCommand(_)) => true, // Can't compare commands
            (SaveSettings { provider: a, model: b, api_key: c }, SaveSettings { provider: d, model: e, api_key: f }) => a == d && b == e && c == f,
            (FetchModels { provider_id: a, api_key: b }, FetchModels { provider_id: c, api_key: d }) => a == c && b == d,
            (Rollback { tool_call_id: a }, Rollback { tool_call_id: b }) => a == b,
            (Interrupt, Interrupt) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Submit(String),
    Command(String),
    Cancel,
    ToolPermission { tool: String, action: PermissionAction },
}

/// Render state containing only the fields needed for rendering.
/// This avoids cloning the entire AppState each frame.
#[derive(Clone)]
pub struct RenderState {
    pub messages: Vec<crate::components::MessageItem>,
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub input_right_info: String,
    pub mode: TuiMode,
    pub running: bool,
    pub show_sidebar: bool,
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub top_bar: TopBarState,
    pub permission_modal: PermissionModalState,
    pub command_palette: CommandPaletteState,
    pub scroll: ScrollState,
    pub animation: AnimationState,
    pub diff_viewer: Option<DiffViewer>,
    pub session_token_usage: TokenUsage,
    pub session_tree: crate::components::SessionTreeNavigator,
    pub background_jobs: Vec<crate::components::status_bar::BackgroundJob>,
    pub onboarding: Option<Onboarding>,
    pub clear_input_confirm: ClearInputConfirm,
    pub model_picker: Option<ModelPicker>,
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<std::time::Instant>,
    pub mock_mode: bool,
}

impl RenderState {
    pub fn from(state: &AppState) -> Self {
        Self {
            messages: state.messages.clone(),
            textarea: state.textarea.clone(),
            input_right_info: state.input_right_info.clone(),
            mode: state.mode.clone(),
            running: state.running,
            show_sidebar: state.show_sidebar,
            agent_running: state.agent_running,
            current_model: state.current_model.clone(),
            top_bar: state.top_bar.clone(),
            permission_modal: state.permission_modal.clone(),
            command_palette: state.command_palette.clone(),
            scroll: state.scroll.clone(),
            animation: state.animation.clone(),
            diff_viewer: state.diff_viewer.clone(),
            session_token_usage: state.session_token_usage.clone(),
            session_tree: state.session_tree.clone(),
            background_jobs: state.background_jobs.clone(),
            onboarding: state.onboarding.clone(),
            clear_input_confirm: state.clear_input_confirm.clone(),
            model_picker: state.model_picker.clone(),
            status_header: state.status_header.clone(),
            status_details: state.status_details.clone(),
            status_start_time: state.status_start_time,
            mock_mode: state.mock_mode,
        }
    }
}

/// Convert AgentEvent to Msg::AgentEvent variant.
impl TryFrom<AgentEvent> for Msg {
    type Error = std::convert::Infallible;
    fn try_from(event: AgentEvent) -> Result<Self, Self::Error> {
        Ok(Msg::AgentEvent(event))
    }
}
