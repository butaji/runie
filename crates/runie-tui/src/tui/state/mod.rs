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

// ─── Extracted modules ─────────────────────────────────────────────────────────

pub mod types;
pub mod enums;

pub use types::*;
pub use enums::*;

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
            thinking_start: None,
            thinking_duration: None,
            is_thinking: false,
            show_sidebar: false,
            terminal_size: (0, 0),
            top_bar: TopBarState::default(),
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
        }
    }
}

// ─── RenderState ──────────────────────────────────────────────────────────────

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
