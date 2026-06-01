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
pub mod overlay;
pub mod system;

// Re-export sub-state types
pub use agent::AgentState;
pub use chat::ChatState;
pub use layout::LayoutState;
pub use overlay::OverlayState;
pub use system::SystemState;

// ─── Extracted modules ─────────────────────────────────────────────────────────

pub mod types;
pub mod enums;

pub use types::*;
pub use enums::*;

// ─── Thinking state ────────────────────────────────────────────────────────────

/// Collapsed thinking state - replaces 4 separate fields
#[derive(Clone, Default)]
pub struct ThinkingState {
    pub start: Option<std::time::Instant>,
    pub text: String,
    /// Accumulated thinking duration before tool interruptions
    pub accrued_duration: Option<std::time::Duration>,
}

// ─── AppState (using sub-states) ──────────────────────────────────────────────

/// AppState is the main application state, decomposed into focused sub-states.
/// 
/// For backward compatibility, fields are kept at the top level AND organized into sub-states.
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
    pub thinking: Option<ThinkingState>,

    pub show_sidebar: bool,
    pub show_thoughts: bool,
    pub terminal_size: (u16, u16),
    pub context: ContextState,

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

    pub top_bar: TopBarState,

    // Turn tracking for global tags display
    pub last_turn_duration_secs: Option<u64>,
    pub last_turn_tokens: Option<usize>,
    pub last_turn_tool_calls: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(),
            scroll: ScrollState::default(),
            input_history: Vec::new(),
            input_history_index: None,
            input_draft: String::new(),
            agent_running: false,
            current_model: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            agent_start_time: None,
            background_jobs: Vec::new(),
            thinking: None,
            show_sidebar: false,
            show_thoughts: false,
            terminal_size: (0, 0),
            context: ContextState::default(),
            running: true,
            mock_mode: false,
            status_header: None,
            status_details: None,
            status_start_time: None,
            clear_input_confirm: ClearInputConfirm::default(),
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            model_picker: None,
            diff_viewer: None,
            session_tree: crate::components::SessionTreeNavigator::new(),
            animation: AnimationState::default(),
            onboarding: None,
            mode: TuiMode::Chat,
            top_bar: TopBarState::default(),
            last_turn_duration_secs: None,
            last_turn_tokens: None,
            last_turn_tool_calls: None,
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


