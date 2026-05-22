use crate::components::{MessageItem, DiffViewer, CommandPalette};
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use crate::components::PermissionAction;
use crate::components::SessionTreeNavigator;
pub use crate::components::onboarding::{Onboarding, OnboardingStep};
use runie_ai::TokenUsage;
use runie_core::SlashCommand;

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
        }
    }
}

#[derive(Clone)]
pub struct PermissionModalState {
    pub tool: Option<String>,
    pub args: Option<String>,
    pub desc: Option<String>,
    pub tool_call_id: Option<String>,
}

impl Default for PermissionModalState {
    fn default() -> Self {
        Self {
            tool: None,
            args: None,
            desc: None,
            tool_call_id: None,
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
    pub input_lines: Vec<String>,
    pub cursor_col: usize,
    pub cursor_row: usize,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
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
    // Input (user typing)
    InsertChar(char),
    Backspace,
    DeleteForward,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorToStart,
    MoveCursorToEnd,
    InsertNewline,
    DeleteWordBackward,
    DeleteToStart,

    // App
    Submit,
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
    OnboardingSubmit,
    OnboardingSkip,
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        use Msg::*;
        match (self, other) {
            (InsertChar(a), InsertChar(b)) => a == b,
            (Backspace, Backspace) => true,
            (DeleteForward, DeleteForward) => true,
            (MoveCursorLeft, MoveCursorLeft) => true,
            (MoveCursorRight, MoveCursorRight) => true,
            (MoveCursorUp, MoveCursorUp) => true,
            (MoveCursorDown, MoveCursorDown) => true,
            (MoveCursorToStart, MoveCursorToStart) => true,
            (MoveCursorToEnd, MoveCursorToEnd) => true,
            (InsertNewline, InsertNewline) => true,
            (DeleteWordBackward, DeleteWordBackward) => true,
            (DeleteToStart, DeleteToStart) => true,
            (Submit, Submit) => true,
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
            (AgentEvent(_), AgentEvent(_)) => true, // Compare by variant only
            (Tick, Tick) => true,
            (CursorBlink, CursorBlink) => true,
            (SlashCommand(_), SlashCommand(_)) => true, // Compare by variant only
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
            (OnboardingSubmit, OnboardingSubmit) => true,
            (OnboardingSkip, OnboardingSkip) => true,
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
    pub input_lines: Vec<String>,
    pub cursor_col: usize,
    pub cursor_row: usize,
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
            input_lines: state.input_lines.clone(),
            cursor_col: state.cursor_col,
            cursor_row: state.cursor_row,
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

