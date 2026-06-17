//! Central event dispatcher.

use crate::event::DialogEvent;
use crate::model::AppState;
use crate::Event;

/// Dispatch an event when no dialog is open and no special early-return
/// handler has consumed it.
pub(crate) fn dispatch_event(state: &mut AppState, event: Event) {
    match event {
        // Input
        Event::Input(_)
        | Event::Backspace
        | Event::Newline
        | Event::Submit
        | Event::Escape
        | Event::CursorLeft
        | Event::CursorRight
        | Event::CursorStart
        | Event::CursorEnd
        | Event::DeleteWord
        | Event::DeleteToEnd
        | Event::DeleteToStart
        | Event::KillChar
        | Event::HistoryPrev
        | Event::HistoryNext
        | Event::Undo
        | Event::Redo
        | Event::CursorWordLeft
        | Event::CursorWordRight
        | Event::PageUp
        | Event::PageDown
        | Event::GoToTop
        | Event::GoToBottom
        | Event::Paste(_)
        | Event::PasteImage
        | Event::MouseClick { .. }
        | Event::MouseRelease { .. }
        | Event::MouseDrag { .. }
        | Event::MouseMove { .. }
        | Event::MouseScrollUp
        | Event::MouseScrollDown
        | Event::FocusGained
        | Event::FocusLost
        | Event::TerminalSize { .. } => super::input::input_event(state, event),
        // Agent
        Event::Thinking { .. }
        | Event::ThoughtDone { .. }
        | Event::ToolStart { .. }
        | Event::ToolEnd { .. }
        | Event::ResponseDelta { .. }
        | Event::Response { .. }
        | Event::TurnComplete { .. }
        | Event::Done { .. }
        | Event::Error { .. } => super::agent::agent_event(state, event),
        // Replay
        Event::MessageReplayed {
            id,
            role,
            content,
            timestamp,
            provider,
        } => {
            state.replay_message(
                id.clone(),
                role.clone(),
                content.clone(),
                timestamp,
                provider.clone(),
            );
        }
        // Scroll
        Event::Up | Event::Down => super::input::scroll_event(state, event),
        // Control
        Event::Quit
        | Event::Reset
        | Event::Abort
        | Event::FollowUp
        | Event::SpawnAgent { .. }
        | Event::SteerAgent { .. }
        | Event::CancelAgent { .. }
        | Event::ToggleExpand
        | Event::Dequeue
        | Event::OpenExternalEditor
        | Event::ExternalEditorDone { .. }
        | Event::ShareSession
        | Event::Suspend
        | Event::ToggleVimMode
        | Event::CopyLastResponse
        | Event::OpenSessionList
        | Event::NewSession
        | Event::ResumeSession
        | Event::SelectSession { .. }
        | Event::StarSession { .. }
        | Event::RenameSession { .. }
        | Event::DeleteSession { .. } => super::system::control_event(state, event),
        // ModelConfig
        Event::SwitchModel { .. }
        | Event::SwitchTheme { .. }
        | Event::CycleModelNext
        | Event::CycleModelPrev
        | Event::ToggleScopedModelsDialog
        | Event::ScopedModelToggle { .. }
        | Event::ScopedModelEnableAll
        | Event::ScopedModelDisableAll
        | Event::ScopedModelToggleProvider { .. }
        | Event::ToggleSettingsDialog
        | Event::SettingsUp
        | Event::SettingsDown
        | Event::SettingsLeft
        | Event::SettingsRight
        | Event::SettingsSelect
        | Event::SettingsClose
        | Event::SettingsSwitchCategory { .. }
        | Event::CycleThinkingLevel
        | Event::SetThinkingLevel(_)
        | Event::ToggleReadOnly
        | Event::TrustProject
        | Event::UntrustProject
        | Event::ReloadAll
        | Event::KeybindingsReloaded => super::agent::model_config_event(state, event),
        // Dialog
        Event::ToggleWelcome
        | Event::ToggleCommandPalette
        | Event::PaletteFilter(_)
        | Event::PaletteBackspace
        | Event::PaletteUp
        | Event::PaletteDown
        | Event::PaletteSelect
        | Event::PaletteClose
        | Event::ToggleModelSelector
        | Event::ModelSelectorFilter(_)
        | Event::ModelSelectorBackspace
        | Event::ModelSelectorUp
        | Event::ModelSelectorDown
        | Event::ModelSelectorSelect
        | Event::ModelSelectorClose
        | Event::TogglePathCompletion
        | Event::PathCompletionUp
        | Event::PathCompletionDown
        | Event::PathCompletionSelect
        | Event::PathCompletionClose
        | Event::CommandFormInput(_)
        | Event::CommandFormBackspace
        | Event::CommandFormUp
        | Event::CommandFormDown
        | Event::CommandFormSubmit
        | Event::CommandFormClose
        | Event::DialogBack
        | Event::ProvidersDialog
        | Event::ProvidersSelectModel { .. }
        | Event::ProvidersDisconnect { .. }
        | Event::ProvidersAdd
        | Event::OpenAgentsManager
        | Event::AgentsManagerSetField { .. }
        | Event::AgentsManagerSave { .. }
        | Event::AgentsManagerDelete { .. }
        | Event::CopyToClipboard(_)
        | Event::CopySelectedBlock
        | Event::CopyBlockMetadata
        | Event::AtFilePicker
        | Event::InsertAtRef(_) => dispatch_dialog_event(state, event),
        // Edit
        Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit => {
            super::tools::update(state, event)
        }
        // System
        Event::SystemMessage { .. }
        | Event::TransientMessage { .. }
        | Event::TransientError { .. }
        | Event::ClearTransient
        | Event::ShowDiagnostics => super::system::handle_system_event(state, event),
        // Session
        Event::ForkSession { .. }
        | Event::CloneSession
        | Event::ToggleSessionTree
        | Event::SessionTreeFilterCycle
        | Event::SessionTreeSelect { .. } => super::session::handle_session_event(state, event),
        // Command
        Event::RunLoadCommand { .. }
        | Event::RunSaveCommand { .. }
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
        | Event::RunPaletteCommand { .. } => super::command::handle_command_event(state, event),
        // LoginFlow
        Event::Start
        | Event::SelectProvider { .. }
        | Event::SubmitKey { .. }
        | Event::ValidationDone { .. }
        | Event::ValidationFailed { .. }
        | Event::ModelsFetched { .. }
        | Event::ToggleModel { .. }
        | Event::Save
        | Event::Cancel => super::login_flow::login_flow_event(state, event),
        _ => {}
    }
}

fn dispatch_dialog_event(state: &mut AppState, event: DialogEvent) {
    if is_toggle_dialog_event(&event) {
        super::dialog::dialog_toggle_event(state, event);
    } else if is_form_dialog_event(&event) {
        super::dialog::handle_form_dialog(state, event);
    } else if let DialogEvent::InsertAtRef(path) = event {
        super::dialog::insert_at_ref(state, &path);
    } else if matches!(event, DialogEvent::DialogBack) {
        handle_dialog_back_no_dialog(state);
    }
}

fn handle_dialog_back_no_dialog(state: &mut AppState) {
    if state.open_dialog.is_none() && state.config.vim_mode {
        state.view.vim_nav_mode = true;
        state.view.selected_post = state.current_bottom_post_index();
        state.mark_dirty();
    }
}

pub(crate) fn is_dialog_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleWelcome
            | Event::ToggleCommandPalette
            | Event::ToggleSettingsDialog
            | Event::ToggleModelSelector
            | Event::ToggleScopedModelsDialog
            | Event::ToggleVimMode
            | Event::AtFilePicker
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
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::SettingsSwitchCategory { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::DialogBack
            | Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::OpenAgentsManager
            | Event::AgentsManagerSetField { .. }
            | Event::AgentsManagerSave { .. }
            | Event::AgentsManagerDelete { .. }
            | Event::CopyToClipboard(_)
            | Event::CopySelectedBlock
            | Event::CopyBlockMetadata
            | Event::InsertAtRef(_)
    )
}

fn is_toggle_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::ToggleWelcome
            | DialogEvent::ToggleCommandPalette
            | DialogEvent::ToggleSettingsDialog
            | DialogEvent::ToggleModelSelector
            | DialogEvent::AtFilePicker
            | DialogEvent::PaletteFilter(_)
            | DialogEvent::PaletteBackspace
            | DialogEvent::PaletteUp
            | DialogEvent::PaletteDown
            | DialogEvent::PaletteSelect
            | DialogEvent::PaletteClose
            | DialogEvent::ModelSelectorFilter(_)
            | DialogEvent::ModelSelectorBackspace
            | DialogEvent::ModelSelectorUp
            | DialogEvent::ModelSelectorDown
            | DialogEvent::ModelSelectorSelect
            | DialogEvent::ModelSelectorClose
            | DialogEvent::TogglePathCompletion
            | DialogEvent::PathCompletionUp
            | DialogEvent::PathCompletionDown
            | DialogEvent::PathCompletionSelect
            | DialogEvent::PathCompletionClose
            | DialogEvent::ToggleVimMode
            | DialogEvent::OpenAgentsManager
            | DialogEvent::AgentsManagerSetField { .. }
            | DialogEvent::AgentsManagerSave { .. }
            | DialogEvent::AgentsManagerDelete { .. }
            | DialogEvent::ProvidersDialog
            | DialogEvent::ProvidersAdd
            | DialogEvent::ProvidersSelectModel { .. }
            | DialogEvent::ProvidersDisconnect { .. }
            | DialogEvent::ToggleScopedModelsDialog
            | DialogEvent::ScopedModelEnableAll
            | DialogEvent::ScopedModelDisableAll
    )
}

fn is_form_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::CommandFormInput(_)
            | DialogEvent::CommandFormBackspace
            | DialogEvent::CommandFormUp
            | DialogEvent::CommandFormDown
            | DialogEvent::CommandFormSubmit
            | DialogEvent::CommandFormClose
    )
}
