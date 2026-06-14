//! Central event dispatcher. Splits the large `update` match into
//! per-category handler functions so the main dispatcher stays small.

use crate::model::AppState;
use crate::Event;

use super::{
    agent, control, dialog, dialog_toggle, edit, input_dispatch, model_config, scroll,
    settings_dialog,
};

/// Dispatch an event when no dialog is open and no special early-return
/// handler has consumed it.
pub fn dispatch_event(state: &mut AppState, event: Event) {
    match event {
        e if is_input_event(&e) => input_dispatch::input_event(state, e),
        e if is_agent_event(&e) => agent::agent_event(state, e),
        e if is_scroll_event(&e) => scroll::scroll_event(state, e),
        e if is_control_event(&e) => control::control_event(state, e),
        e if is_model_config_event(&e) => model_config::model_config_event(state, e),
        e if is_dialog_toggle_event(&e) => dialog_toggle::dialog_toggle_event(state, e),
        e if is_form_event(&e) => dialog::handle_form_dialog(state, e),
        e if is_edit_event(&e) => edit::update(state, e),
        e => dispatch_misc_event(state, e),
    }
}

fn is_input_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Input(_)
            | Event::Backspace
            | Event::Newline
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::Undo
            | Event::Redo
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::Paste(_)
            | Event::PasteImage
            | Event::Submit
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::InsertAtRef(_)
    )
}

fn is_agent_event(event: &Event) -> bool {
    matches!(
        event,
        Event::AgentThinking { .. }
            | Event::AgentThoughtDone { .. }
            | Event::AgentToolStart { .. }
            | Event::AgentToolEnd { .. }
            | Event::AgentResponse { .. }
            | Event::AgentTurnComplete { .. }
            | Event::AgentDone { .. }
            | Event::AgentError { .. }
    )
}

fn is_scroll_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ScrollUp
            | Event::ScrollDown
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom
    )
}

fn is_control_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Quit
            | Event::Reset
            | Event::Abort
            | Event::ExternalEditorDone { .. }
            | Event::SpawnAgent { .. }
            | Event::Suspend
            | Event::ShareSession
            | Event::OpenExternalEditor
            | Event::ToggleExpand
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::ForkSession { .. }
            | Event::CloneSession
            | Event::SessionTreeSelect { .. }
            | Event::CopyToClipboard(_)
            | Event::CopyLastResponse
    )
}

fn is_model_config_event(event: &Event) -> bool {
    matches!(
        event,
        Event::SwitchModel { .. }
            | Event::SwitchTheme { .. }
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::FollowUp
            | Event::Dequeue
            | Event::ToggleVimMode
    )
}

fn is_dialog_toggle_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleCommandPalette
            | Event::ToggleModelSelector
            | Event::ToggleScopedModelsDialog
            | Event::ScopedModelToggle { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::ScopedModelToggleProvider { .. }
            | Event::AtFilePicker
            | Event::ToggleSettingsDialog
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::PaletteFilter(_)
            | Event::PaletteBackspace
            | Event::PaletteUp
            | Event::PaletteDown
            | Event::PaletteSelect
            | Event::PaletteClose
            | Event::ModelSelectorFilter(_)
            | Event::ModelSelectorBackspace
            | Event::ModelSelectorUp
            | Event::ModelSelectorDown
            | Event::ModelSelectorSelect
            | Event::ModelSelectorClose
    )
}

fn is_form_event(event: &Event) -> bool {
    matches!(
        event,
        Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
    )
}

fn is_edit_event(event: &Event) -> bool {
    matches!(
        event,
        Event::PendingEdit { .. }
            | Event::ApproveEdit
            | Event::RejectEdit
            | Event::ReloadAll
            | Event::ShowDiagnostics
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::RunSaveCommand { .. }
            | Event::RunLoadCommand { .. }
            | Event::RunDeleteCommand { .. }
            | Event::RunImportCommand { .. }
            | Event::RunExportCommand { .. }
            | Event::RunSkillCommand { .. }
            | Event::RunLoginCommand { .. }
            | Event::RunLogoutCommand { .. }
            | Event::RunNameCommand { .. }
            | Event::RunForkCommand { .. }
            | Event::RunCompactCommand { .. }
            | Event::RunPromptCommand { .. }
            | Event::RunThinkingCommand { .. }
            | Event::RunPaletteCommand { .. }
    )
}

fn dispatch_misc_event(state: &mut AppState, event: Event) {
    if dispatch_core_misc_event(state, &event) {
        return;
    }
    dispatch_remaining_misc_event(state, event);
}

fn dispatch_core_misc_event(state: &mut AppState, event: &Event) -> bool {
    match event {
        Event::SystemMessage { content } => state.add_system_msg(content.clone()),
        Event::TransientMessage { content, level } => state.set_transient(content.clone(), *level),
        Event::TransientError { content } => {
            state.set_transient(content.clone(), crate::event::TransientLevel::Error)
        }
        Event::ClearTransient => state.clear_transient(),
        Event::TerminalSize { width, height } => {
            state.set_last_content_width(*width);
            state.set_last_visible_height(*height);
            state.mark_dirty();
        }
        Event::SettingsSwitchCategory { category } => {
            settings_dialog::handle_settings_category(state, *category)
        }
        _ => return false,
    }
    true
}

fn dispatch_remaining_misc_event(state: &mut AppState, event: Event) {
    match event {
        Event::OpenAgentsManager
        | Event::AgentsManagerSetField { .. }
        | Event::AgentsManagerSave { .. }
        | Event::AgentsManagerDelete { .. } => {
            crate::commands::agents_manager::agents_manager_event(state, event)
        }
        Event::MouseClick { .. }
        | Event::MouseRelease { .. }
        | Event::MouseDrag { .. }
        | Event::MouseMove { .. }
        | Event::FocusGained
        | Event::FocusLost
        | Event::DialogBack => {}
        Event::ProvidersDialog
        | Event::ProvidersSelectModel { .. }
        | Event::ProvidersDisconnect { .. }
        | Event::ProvidersAdd
        | Event::LoginFlowStart
        | Event::LoginFlowSelectProvider { .. }
        | Event::LoginFlowSubmitKey { .. }
        | Event::LoginFlowValidationDone { .. }
        | Event::LoginFlowValidationFailed { .. }
        | Event::LoginFlowModelsFetched { .. }
        | Event::LoginFlowToggleModel { .. }
        | Event::LoginFlowSave
        | Event::LoginFlowCancel => unreachable!(),
        _ => {}
    }
}
