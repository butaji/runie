//! AppState and related types.
//!
//! Phase 2 of architecture migration: decompose monolithic AppState into focused sub-states.

use crate::components::{DiffViewer, PaletteCommand, ModelPicker, ExtensionsModal};
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
    pub history_search_query: String,
    pub history_search_matches: Vec<usize>,
    pub history_search_index: usize,

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
    pub permission_mode: crate::tui::state::PermissionMode,
    pub allowed_tools: std::collections::HashSet<String>,
    pub allowed_categories: std::collections::HashSet<String>,
    pub command_palette: CommandPaletteState,
    pub slash_menu: crate::components::SlashMenu,
    pub shortcuts_panel: crate::components::ShortcutsPanel,
    pub settings_modal: crate::components::SettingsModal,
    pub home_screen: crate::components::HomeScreen,
    pub file_picker: crate::components::FilePicker,
    pub plan_modal: crate::components::PlanModal,
    pub context_usage_modal: crate::components::ContextUsageModal,
    pub model_picker: Option<ModelPicker>,
    pub extensions_modal: Option<ExtensionsModal>,
    pub diff_viewer: Option<DiffViewer>,
    pub session_tree: crate::components::SessionTreeNavigator,

    pub animation: AnimationState,
    pub onboarding: Option<Onboarding>,
    pub mode: TuiMode,

    pub top_bar: TopBarState,
    pub current_theme: String,

    // Turn tracking for global tags display
    pub last_turn_duration_secs: Option<u64>,
    pub last_turn_tokens: Option<usize>,
    pub last_turn_tool_calls: Option<usize>,
    pub turn_success: Option<bool>,

    // Extension registry for plugins
    pub extension_registry: std::sync::Arc<runie_ext::ExtensionRegistry>,
    pub questionnaire: Option<crate::components::questionnaire_panel::QuestionnaireState>,
    pub subagent_panel: crate::components::subagent_panel::SubagentPanel,

    // Fullscreen viewer state
    pub fullscreen_content: Option<String>,
    pub fullscreen_scroll_offset: usize,

    // Layout mode for responsive terminal display
    pub layout_mode: LayoutMode,

    // Animation configuration
    pub animation_config: AnimationConfig,

    // UI toggles
    pub compact_mode: bool,
    pub multiline_input: bool,

    // Session starting indicator (shows "⠼ Starting session… X.Xs" after HomeScreen transition)
    pub session_starting: Option<std::time::Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        let ui = default_ui_components();
        Self {
            messages: Vec::new(), textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(), scroll: ScrollState::default(),
            input_history: Vec::new(), input_history_index: None,
            input_draft: String::new(), history_search_query: String::new(),
            history_search_matches: Vec::new(), history_search_index: 0,
            agent_running: false, current_model: None,
            token_usage: TokenUsage::default(), session_token_usage: TokenUsage::default(),
            agent_start_time: None, background_jobs: Vec::new(), thinking: None,
            show_sidebar: false, show_thoughts: false, terminal_size: (0, 0),
            context: ContextState::default(), running: true, mock_mode: false,
            status_header: None, status_details: None, status_start_time: None,
            clear_input_confirm: ClearInputConfirm::default(),
            permission_modal: PermissionModalState::default(),
            permission_mode: PermissionMode::AutoApprove, allowed_tools: std::collections::HashSet::new(),
            allowed_categories: std::collections::HashSet::new(),
            command_palette: ui.0,
            slash_menu: ui.1,
            shortcuts_panel: ui.2,
            settings_modal: ui.3,
            home_screen: ui.4,
            file_picker: ui.5,
            plan_modal: ui.6,
            context_usage_modal: ui.7,
            model_picker: None, extensions_modal: None, diff_viewer: None,
            session_tree: crate::components::SessionTreeNavigator::new(),
            animation: AnimationState::default(), onboarding: None,
            mode: TuiMode::HomeScreen, top_bar: TopBarState::default(),
            current_theme: "crush_grok".to_string(),
            last_turn_duration_secs: None, last_turn_tokens: None,
            last_turn_tool_calls: None, turn_success: None,
            extension_registry: std::sync::Arc::new(runie_ext::ExtensionRegistry::new()),
            questionnaire: None, subagent_panel: crate::components::subagent_panel::SubagentPanel::new(),
            fullscreen_content: None, fullscreen_scroll_offset: 0,
            layout_mode: LayoutMode::Fullscreen, animation_config: AnimationConfig::default(),
            compact_mode: false, multiline_input: false, session_starting: None,
        }
    }
}

fn default_ui_components() -> (
    CommandPaletteState,
    crate::components::SlashMenu,
    crate::components::ShortcutsPanel,
    crate::components::SettingsModal,
    crate::components::HomeScreen,
    crate::components::FilePicker,
    crate::components::PlanModal,
    crate::components::ContextUsageModal,
) {
    (
        CommandPaletteState::default(),
        crate::components::SlashMenu::new(),
        crate::components::ShortcutsPanel::new(),
        crate::components::SettingsModal::new(),
        crate::components::HomeScreen::new(),
        crate::components::FilePicker::new(),
        crate::components::PlanModal::new(),
        crate::components::ContextUsageModal::new(),
    )
}

/// Convert AgentEvent to Msg::AgentEvent variant.
impl TryFrom<AgentEvent> for Msg {
    type Error = std::convert::Infallible;
    fn try_from(event: AgentEvent) -> Result<Self, Self::Error> {
        Ok(Msg::AgentEvent(event))
    }
}


