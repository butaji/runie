//! Mode transition tests for runie-tui.
//!
//! Comprehensive tests for all mode transitions:
//! - Chat ↔ CommandPalette
//! - Chat ↔ Overlay
//! - Chat ↔ Permission
//! - Chat ↔ Onboarding
//! - Chat ↔ SessionTree
//! - State preservation across transitions
//! - Paste blocking in blocking modes
//! - Global hotkey behavior

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use crate::components::CommandPalette;
use crate::tui::state::{AppState, Msg, TuiMode, ScrollState, ContextState, PermissionModalState, CommandPaletteState, AnimationState, TopBarState, ClearInputConfirm, OnboardingStep};
use crate::tui::update::update;
use crate::tui::events::event_to_msg;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, PermissionDecision, TokenUsage as AgentTokenUsage};
use runie_ai::TokenUsage as AiTokenUsage;
use crate::components::MessageItem;
use crate::components::SessionTreeNavigator;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui_textarea::{TextArea, Input, Key};

// ═══════════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a default AppState for testing.
pub fn make_state() -> AppState {
    AppState {
        messages: Vec::new(),
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("gpt-4".to_string()),
        context: ContextState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None, agent_start_time: None,
        input_history: Vec::new(), input_history_index: None,
        input_draft: String::new(),
        status_header: None, status_details: None, status_start_time: None,
        thinking: None, mock_mode: false,
        top_bar: TopBarState::default(),
        last_turn_duration_secs: None, last_turn_tokens: None,
        last_turn_tool_calls: None, turn_success: None,
        slash_menu: crate::components::SlashMenu::new(),
        shortcuts_panel: crate::components::ShortcutsPanel::new(),
        show_thoughts: false,
    }
}

/// Create AppState with text in textarea.
pub fn make_state_with_text(text: &str) -> AppState {
    AppState {
        messages: Vec::new(),
        textarea: TextArea::new(vec![text.to_string()]),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("gpt-4".to_string()),
        context: ContextState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None, agent_start_time: None,
        input_history: Vec::new(), input_history_index: None,
        input_draft: String::new(),
        status_header: None, status_details: None, status_start_time: None,
        thinking: None, mock_mode: false,
        top_bar: TopBarState::default(),
        last_turn_duration_secs: None, last_turn_tokens: None,
        last_turn_tool_calls: None, turn_success: None,
        slash_menu: crate::components::SlashMenu::new(),
        shortcuts_panel: crate::components::ShortcutsPanel::new(),
        show_thoughts: false,
    }
}

/// Create AppState with messages.
pub fn make_state_with_messages(messages: Vec<MessageItem>) -> AppState {
    AppState {
        messages,
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("gpt-4".to_string()),
        context: ContextState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None, agent_start_time: None,
        input_history: Vec::new(), input_history_index: None,
        input_draft: String::new(),
        status_header: None, status_details: None, status_start_time: None,
        thinking: None, mock_mode: false,
        top_bar: TopBarState::default(),
        last_turn_duration_secs: None, last_turn_tokens: None,
        last_turn_tool_calls: None, turn_success: None,
        slash_menu: crate::components::SlashMenu::new(),
        shortcuts_panel: crate::components::ShortcutsPanel::new(),
        show_thoughts: false,
    }
}

/// Create AppState in a specific mode.
pub fn make_state_in_mode(mode: TuiMode) -> AppState {
    AppState {
        mode,
        current_model: Some("gpt-4".to_string()),
        ..Default::default()
    }
}

/// Enter a mode directly via Msg.
pub fn enter_mode(state: &mut AppState, palette: &mut CommandPalette, mode: TuiMode) {
    match mode {
        TuiMode::Chat => {
            state.mode = TuiMode::Chat;
        }
        TuiMode::CommandPalette => {
            update(state, palette, Msg::OpenCommandPalette);
        }
        TuiMode::Overlay => {
            // Overlay is entered via model picker or similar
            state.mode = TuiMode::Overlay;
            state.model_picker = Some(crate::components::ModelPicker::with_default_models());
        }
        TuiMode::Permission => {
            state.mode = TuiMode::Permission;
            state.permission_modal.tool = Some("bash".to_string());
            state.permission_modal.tool_call_id = Some("test_tool".to_string());
        }
        TuiMode::Onboarding => {
            update(state, palette, Msg::EnterOnboarding);
        }
        TuiMode::SessionTree => {
            state.session_tree.toggle();
            state.mode = TuiMode::SessionTree;
        }
        TuiMode::DiffViewer => {
            state.mode = TuiMode::DiffViewer;
            state.diff_viewer = Some(crate::components::DiffViewer::new(
                "test.txt".to_string(),
                "old content".to_string(),
                "new content".to_string(),
            ));
        }
        TuiMode::Select => {
            state.mode = TuiMode::Select;
        }
    }
}

/// Helper to simulate a key event and convert to Msg.
pub fn simulate_key(code: KeyCode, modifiers: KeyModifiers, mode: TuiMode) -> Option<Msg> {
    let event = Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = AppState {
        mode,
        ..Default::default()
    };
    event_to_msg(event, &state).into_iter().next()
}

/// Helper to simulate a paste event.
pub fn simulate_paste(text: &str, mode: TuiMode) -> Vec<Msg> {
    let event = Event::Paste(text.to_string());
    let state = AppState {
        mode,
        ..Default::default()
    };
    event_to_msg(event, &state)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUB-MODULES
// ═══════════════════════════════════════════════════════════════════════════════

mod chat_to_palette;
mod chat_to_overlay;
mod chat_to_permission;
mod chat_to_onboarding;
mod chat_to_session_tree;
mod state_preservation;
mod paste_blocking;
mod global_hotkeys;

pub use chat_to_palette::*;
pub use chat_to_overlay::*;
pub use chat_to_permission::*;
pub use chat_to_onboarding::*;
pub use chat_to_session_tree::*;
pub use state_preservation::*;
pub use paste_blocking::*;
pub use global_hotkeys::*;