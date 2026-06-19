//! Central event dispatcher.

use crate::event::DialogEvent;
use crate::model::AppState;
use crate::Event;

/// Dispatch an event when no dialog is open and no special early-return
/// handler has consumed it.
pub(crate) fn dispatch_event(state: &mut AppState, event: Event) {
    if let Event::MessageReplayed {
        id,
        role,
        content,
        timestamp,
        provider,
    } = &event
    {
        state.replay_message(
            id.clone(),
            role.clone(),
            content.clone(),
            *timestamp,
            provider.clone(),
        );
        return;
    }
    match categorize(&event) {
        EventCategory::Input => super::input::input_event(state, event),
        EventCategory::Agent => super::agent::agent_event(state, event),
        EventCategory::Scroll => super::input::scroll_event(state, event),
        EventCategory::Control => super::system::control_event(state, event),
        EventCategory::ModelConfig => super::agent::model_config_event(state, event),
        EventCategory::Dialog => dispatch_dialog_event(state, event),
        EventCategory::Edit => super::tools::update(state, event),
        EventCategory::System => super::system::handle_system_event(state, event),
        EventCategory::Session => super::session::handle_session_event(state, event),
        EventCategory::Command => super::command::handle_command_event(state, event),
        EventCategory::LoginFlow => super::login_flow::login_flow_event(state, event),
        EventCategory::Other => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventCategory {
    Input,
    Agent,
    Scroll,
    Control,
    ModelConfig,
    Dialog,
    Edit,
    System,
    Session,
    Command,
    LoginFlow,
    Other,
}

fn categorize(event: &Event) -> EventCategory {
    if let Some(cat) = categorize_input_agent_scroll(event) {
        return cat;
    }
    if let Some(cat) = categorize_control_model_dialog(event) {
        return cat;
    }
    if let Some(cat) = categorize_edit_system_session(event) {
        return cat;
    }
    if let Some(cat) = categorize_command_login(event) {
        return cat;
    }
    EventCategory::Other
}

fn categorize_input_agent_scroll(event: &Event) -> Option<EventCategory> {
    if is_input_event(event) {
        return Some(EventCategory::Input);
    }
    if is_agent_event(event) {
        return Some(EventCategory::Agent);
    }
    if matches!(event, Event::Up | Event::Down) {
        return Some(EventCategory::Scroll);
    }
    None
}

fn is_input_event(event: &Event) -> bool {
    matches!(
        event,
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
            | Event::TerminalSize { .. }
    )
}

fn is_agent_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolStart { .. }
            | Event::ToolEnd { .. }
            | Event::ResponseDelta { .. }
            | Event::Response { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::Error { .. }
    )
}

fn categorize_control_model_dialog(event: &Event) -> Option<EventCategory> {
    if is_control_event(event) {
        return Some(EventCategory::Control);
    }
    if is_model_config_event(event) {
        return Some(EventCategory::ModelConfig);
    }
    if is_dialog_category_event(event) {
        return Some(EventCategory::Dialog);
    }
    None
}

fn is_control_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Quit
            | Event::ForceQuit
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
            | Event::DeleteSession { .. }
    )
}

fn is_model_config_event(event: &Event) -> bool {
    matches!(
        event,
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
            | Event::KeybindingsReloaded
    )
}

fn is_dialog_category_event(event: &Event) -> bool {
    is_palette_selector_event(event)
        || is_path_form_event(event)
        || matches!(
            event,
            Event::ToggleWelcome
                | Event::DialogBack
                | Event::ProvidersDialog
                | Event::ProvidersSelectModel { .. }
                | Event::ProvidersDisconnect { .. }
                | Event::ProvidersAdd
                | Event::ProviderEditModels { .. }
                | Event::ProviderEditModelsToggle { .. }
                | Event::ProviderEditModelsSave { .. }
                | Event::ProviderEditModelsClose
                | Event::OpenAgentsManager
                | Event::AgentsManagerSetField { .. }
                | Event::AgentsManagerSave { .. }
                | Event::AgentsManagerDelete { .. }
                | Event::CopyToClipboard(_)
                | Event::CopySelectedBlock
                | Event::CopyBlockMetadata
                | Event::AtFilePicker
                | Event::InsertAtRef(_)
        )
}

fn is_palette_selector_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleCommandPalette
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
    )
}

fn is_path_form_event(event: &Event) -> bool {
    matches!(
        event,
        Event::TogglePathCompletion
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
    )
}

fn categorize_edit_system_session(event: &Event) -> Option<EventCategory> {
    match event {
        Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit => {
            Some(EventCategory::Edit)
        }
        Event::SystemMessage { .. }
        | Event::TransientMessage { .. }
        | Event::TransientError { .. }
        | Event::ClearTransient
        | Event::ShowDiagnostics => Some(EventCategory::System),
        Event::ForkSession { .. }
        | Event::CloneSession
        | Event::ToggleSessionTree
        | Event::SessionTreeFilterCycle
        | Event::SessionTreeSelect { .. } => Some(EventCategory::Session),
        _ => None,
    }
}

fn categorize_command_login(event: &Event) -> Option<EventCategory> {
    match event {
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
        | Event::RunPaletteCommand { .. } => Some(EventCategory::Command),
        Event::Start
        | Event::SelectProvider { .. }
        | Event::SubmitKey { .. }
        | Event::ValidationDone { .. }
        | Event::ValidationFailed { .. }
        | Event::ModelsFetched { .. }
        | Event::ToggleModel { .. }
        | Event::Save
        | Event::Cancel => Some(EventCategory::LoginFlow),
        _ => None,
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
    is_toggle_dialog_event(event)
        || is_form_dialog_event(event)
        || matches!(event, Event::InsertAtRef(_))
        || matches!(event, Event::DialogBack)
}

fn is_toggle_dialog_event(event: &DialogEvent) -> bool {
    is_palette_selector_event(event)
        || is_path_form_event(event)
        || is_agents_manager_event(event)
        || is_provider_edit_models_event(event)
        || matches!(
            event,
            DialogEvent::ToggleWelcome
                | DialogEvent::ToggleSettingsDialog
                | DialogEvent::ToggleModelSelector
                | DialogEvent::AtFilePicker
                | DialogEvent::ToggleVimMode
                | DialogEvent::ProvidersDialog
                | DialogEvent::ProvidersAdd
                | DialogEvent::ProvidersSelectModel { .. }
                | DialogEvent::ProvidersDisconnect { .. }
                | DialogEvent::ProviderEditModels { .. }
                | DialogEvent::ToggleScopedModelsDialog
                | DialogEvent::ScopedModelEnableAll
                | DialogEvent::ScopedModelDisableAll
        )
}

fn is_agents_manager_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::OpenAgentsManager
            | DialogEvent::AgentsManagerSetField { .. }
            | DialogEvent::AgentsManagerSave { .. }
            | DialogEvent::AgentsManagerDelete { .. }
    )
}

fn is_provider_edit_models_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::ProviderEditModels { .. }
            | DialogEvent::ProviderEditModelsToggle { .. }
            | DialogEvent::ProviderEditModelsSave { .. }
            | DialogEvent::ProviderEditModelsClose
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
