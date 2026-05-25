use crate::components::{MessageItem, DiffViewer, PaletteCommand};
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use crate::components::PermissionAction;
use crate::components::SessionTreeNavigator;
pub use crate::components::onboarding::{Onboarding, OnboardingStep};
pub use runie_ai::model_fetcher::ModelInfo;
use runie_ai::TokenUsage;
use runie_core::SlashCommand;
use crossterm::event::KeyEvent;

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
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub agent_count: Option<usize>,
    pub context_badges: Vec<String>,
    pub context_pct: Option<f32>,
    pub context_bar_pct: Option<f32>,
}

impl Default for TopBarState {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            checks_passed: None,
            checks_total: None,
            percentage: None,
            agent_count: None,
            context_badges: Vec::new(),
            context_pct: None,
            context_bar_pct: None,
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
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            feed_offset: 0,
            diff_offset: 0,
            tree_offset: 0,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<MessageItem>,
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
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub session_tree: SessionTreeNavigator,
    pub background_jobs: Vec<crate::components::status_bar::BackgroundJob>,
    pub onboarding: Option<Onboarding>,
    pub terminal_size: (u16, u16),
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar: TopBarState::default(),
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            scroll: ScrollState::default(),
            animation: AnimationState::default(),
            diff_viewer: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(),
            background_jobs: Vec::new(),
            onboarding: None,
            terminal_size: (0, 0),
        }
    }
}

// ─── Standalone Widget Render Functions ────────────────────────────────────────
// These render directly from AppState (no widget instances stored in Tui)

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

    // Input
    ClearInput,
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
            (ClearChat, ClearChat) => true,
            (DirectCommand(a), DirectCommand(b)) => a == b,
            (Paste(a), Paste(b)) => a == b,
            (ModelsFetched(a), ModelsFetched(b)) => a == b,
            (ModelsFetchFailed(a), ModelsFetchFailed(b)) => a == b,
            (Resize(a_w, a_h), Resize(b_w, b_h)) => a_w == b_w && a_h == b_h,
            (Stop, Stop) => true,
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
    SaveSession { name: Option<String> },
    LoadSession { name: String },
    SlashCommand(SlashCommand),
    SaveSettings { provider: String, model: String, api_key: String },
    FetchModels { provider_id: String, api_key: String },
    // P1-4 FIX: Rollback — reverts partial tool changes on permission cancel
    Rollback { tool_call_id: String },
    // P0-1 FIX: Interrupt — cancels the running agent task
    Interrupt,
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
    pub messages: Vec<MessageItem>,
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
    pub session_tree: SessionTreeNavigator,
    pub background_jobs: Vec<crate::components::status_bar::BackgroundJob>,
    pub onboarding: Option<Onboarding>,
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
        }
    }
}

