use crate::components::{MessageItem, DiffViewer, CommandPalette};
use runie_agent::events::{AgentEvent, AgentMessage, PermissionDecision};
use crate::components::PermissionAction;
use crate::tui::update::update;
use crate::components::SessionTreeNavigator;
use runie_ai::TokenUsage;
use runie_core::SlashCommand;

#[derive(Clone)]
pub struct AnimationState {
    pub braille_frame: usize,
    pub streaming_cursor_visible: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            braille_frame: 0,
            streaming_cursor_visible: true,
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
    pub top_bar_repo: String,
    pub top_bar_branch: String,
    pub top_bar_path: String,
    pub top_bar_checks_passed: Option<usize>,
    pub top_bar_checks_total: Option<usize>,
    pub top_bar_percentage: Option<f32>,
    pub top_bar_agent_count: Option<usize>,
    pub permission_modal_tool: Option<String>,
    pub permission_modal_tool_call_id: Option<String>,
    pub permission_modal_args: Option<String>,
    pub permission_modal_desc: Option<String>,
    pub action_log: Vec<Msg>,         // NEW: history of all actions for time-travel debugging
    pub action_log_capacity: usize,    // NEW: max actions to keep (default 1000)
    pub command_palette_open: bool,
    pub command_palette_filter: String,
    pub command_palette_selected: usize,
    pub feed_scroll_offset: usize,
    pub diff_scroll_offset: usize,
    pub tree_scroll_offset: usize,
    pub animation: AnimationState,
    pub diff_viewer: Option<DiffViewer>,
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub session_tree: SessionTreeNavigator,
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
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            top_bar_checks_passed: None,
            top_bar_checks_total: None,
            top_bar_percentage: None,
            top_bar_agent_count: None,
            permission_modal_tool: None,
            permission_modal_tool_call_id: None,
            permission_modal_args: None,
            permission_modal_desc: None,
            action_log: Vec::new(),
            action_log_capacity: 1000,
            command_palette_open: false,
            command_palette_filter: String::new(),
            command_palette_selected: 0,
            feed_scroll_offset: 0,
            diff_scroll_offset: 0,
            tree_scroll_offset: 0,
            animation: AnimationState::default(),
            diff_viewer: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(),
        }
    }
}

impl AppState {
    /// Replay actions from scratch up to index n (time-travel debugging)
    pub fn replay_to(&self, n: usize) -> AppState {
        let mut new_state = AppState::default();
        for i in 0..n.min(self.action_log.len()) {
            update(&mut new_state, self.action_log[i].clone());
        }
        new_state
    }

    /// Get action log as readable strings for debugging
    pub fn action_log_summary(&self) -> Vec<String> {
        self.action_log.iter()
            .enumerate()
            .map(|(i, msg)| format!("{:4}: {:?}", i, msg))
            .collect()
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Submit(String),
    Command(String),
    Cancel,
    ToolPermission { tool: String, action: PermissionAction },
}

