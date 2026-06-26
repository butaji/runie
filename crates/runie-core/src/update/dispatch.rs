//! Central event dispatcher.

use crate::actors::turn::TurnMsg;
use crate::model::AppState;
use crate::Event;

pub(crate) fn dispatch_event(state: &mut AppState, event: Event) {
    if try_handle_early_events(state, &event) { return; }
    match categorize(&event) {
        EventCategory::Input => super::input::input_event(state, event),
        EventCategory::Agent => handle_agent_event(state, event),
        EventCategory::Scroll => super::input::scroll_event(state, event),
        EventCategory::Control => super::system::control_event(state, event),
        EventCategory::ModelConfig => super::agent::model_config_event(state, event),
        EventCategory::Dialog => dispatch_dialog_event(state, event),
        EventCategory::Edit => super::tools::update(state, event),
        EventCategory::System => super::system::handle_system_event(state, event),
        EventCategory::Session => super::session::handle_session_event(state, event),
        EventCategory::Command => super::command::handle_command_event(state, event),
        EventCategory::LoginFlow => crate::login_flow::login_flow_event(state, event),
        EventCategory::Permission => super::permission::permission_event(state, event),
        EventCategory::Other => {}
    }
}

fn try_handle_early_events(state: &mut AppState, event: &Event) -> bool {
    if let Event::MessageReplayed { id, role, content, timestamp, provider } = event {
        state.replay_message(id.clone(), role.clone(), content.clone(), *timestamp, provider.clone());
        return true;
    }
    if let Event::SetPrompt { name } = event {
        state.input_mut().current_prompt = name.clone();
        return true;
    }
    handle_turn_events(state, event) || handle_persistence_events(state, event) || handle_session_store_events(state, event) || handle_io_events(state, event)
}

fn handle_turn_events(state: &mut AppState, event: &Event) -> bool {
    match event {
        Event::TurnAborted => { state.apply_turn_aborted(); true }
        Event::QueueAborted { content } => { state.apply_queue_aborted(content.clone()); true }
        Event::TurnStarted { .. } => { state.apply_turn_started(); true }
        Event::TurnCompleted => { state.apply_turn_completed(); true }
        Event::TurnErrored { .. } => { state.apply_turn_errored(); true }
        Event::TokenStatsUpdated { tokens_in, tokens_out, speed_tps } => {
            state.apply_token_stats(*tokens_in, *tokens_out, *speed_tps);
            true
        }
        // Agent events go through handle_agent_event for session message manipulation
        _ => false,
    }
}

/// Route agent events through TurnActor and handle facts synchronously.
fn handle_agent_event(state: &mut AppState, event: Event) {
    // Send to TurnActor if available
    if let Some(ref handles) = state.actor_handles() {
        if let Some(ref turn) = handles.turn {
            if let Some(turn_msg) = to_turn_msg(&event) {
                turn.try_send(turn_msg);
            }
        }
    }
    // Also handle the event directly for immediate UI updates
    super::agent::agent_event(state, event);
}

/// Convert Event to TurnMsg for routing through TurnActor.
fn to_turn_msg(event: &Event) -> Option<TurnMsg> {
    match event {
        Event::Thinking { id } => Some(TurnMsg::Thinking { id: id.clone() }),
        Event::ThoughtDone { id } => Some(TurnMsg::ThoughtDone { id: id.clone() }),
        Event::ToolStart { id, name, .. } => Some(TurnMsg::ToolStart { id: id.clone(), name: name.clone() }),
        Event::ToolEnd { id, duration_secs, output } => Some(TurnMsg::ToolEnd { id: id.clone(), duration_secs: *duration_secs, output: output.clone() }),
        Event::ResponseDelta { id, content } => Some(TurnMsg::ResponseDelta { id: id.clone(), content: content.clone() }),
        Event::TurnComplete { id, duration_secs } => Some(TurnMsg::TurnComplete { id: id.clone(), duration_secs: *duration_secs }),
        Event::Done { id } => Some(TurnMsg::Done { id: id.clone() }),
        Event::Error { id, message } => Some(TurnMsg::Error { id: id.clone(), message: message.clone() }),
        _ => None,
    }
}

fn handle_persistence_events(state: &mut AppState, event: &Event) -> bool {
    use crate::event::TransientLevel;
    match event {
        Event::TrustLoaded { decisions } => { state.set_trust_decisions(decisions.clone()); true }
        Event::TrustChanged { path, decision } => {
            state.set_trust_decision(path.clone(), *decision);
            // Update read_only based on trust decision (mirrors TrustActor logic).
            // This keeps unit tests synchronous; TrustActor also emits ReadOnlyChanged.
            let new_read_only = !matches!(decision, crate::trust::TrustDecision::Trusted);
            state.config_mut().read_only = new_read_only;
            // When project is trusted, remove the welcome message and notify user
            if matches!(decision, crate::trust::TrustDecision::Trusted) {
                state.session_mut().messages.retain(|m| m.id != "trust_welcome");
                state.messages_changed();
                state.notify(format!("Project '{}' trusted. Read-only disabled.", path.display()), TransientLevel::Success);
            } else {
                state.notify(format!("Project '{}' untrusted. Read-only enabled.", path.display()), TransientLevel::Warning);
            }
            true
        }
        Event::ReadOnlyChanged { enabled } => { state.config_mut().read_only = *enabled; true }
        Event::HistoryLoaded { entries } => {
            // Route through InputActor.
            if let Some(ref handles) = state.actor_handles() {
                handles.try_send_input(crate::actors::InputMsg::HistoryLoaded {
                    entries: entries.clone(),
                });
            }
            true
        }
        _ => false,
    }
}

fn handle_session_store_events(state: &mut AppState, event: &Event) -> bool {
    use crate::event::TransientLevel;
    match event {
        Event::SessionLoaded { name, events, metadata } => { apply_session_loaded(state, name, events, metadata); true }
        Event::SessionSaved { name } => { state.notify(format!("Session '{}' saved.", name), TransientLevel::Info); true }
        Event::SessionDeleted { name } => { state.notify(format!("Session '{}' deleted.", name), TransientLevel::Info); true }
        Event::SessionImported { session } => { apply_session_imported(state, session); true }
        Event::SessionExported { path } => { state.notify(format!("Session exported to '{}'.", path), TransientLevel::Info); true }
        Event::SessionList { sessions } => { apply_session_list(state, sessions); true }
        Event::SessionOperationFailed { operation, error } => { state.notify(format!("{} failed: {}", operation, error), TransientLevel::Error); true }
        _ => false,
    }
}

fn apply_session_loaded(state: &mut AppState, name: &str, events: &[crate::event::DurableCoreEvent], metadata: &Option<Box<crate::session::index::SessionMetadata>>) {
    crate::session::replay::replay_events(state, events);
    if let Some(meta) = metadata {
        state.session_mut().session_display_name = Some(meta.display_name.clone());
        state.session_mut().session_created_at = meta.created_at;
        state.session_mut().session_updated_at = meta.updated_at;
    }
    state.configure_token_tracker();
    state.messages_changed();
    state.notify(format!("Session '{}' loaded.", name), crate::event::TransientLevel::Info);
}

fn apply_session_imported(state: &mut AppState, session: &crate::session::Session) {
    state.restore_session(session);
    state.notify(format!("Session imported from '{}'.", session.name), crate::event::TransientLevel::Info);
}

fn apply_session_list(state: &mut AppState, sessions: &[String]) {
    let content = if sessions.is_empty() {
        "No saved sessions. Use /save name to create one.".into()
    } else {
        format!("Saved sessions:\n{}", sessions.join("\n"))
    };
    state.notify(content, crate::event::TransientLevel::Info);
}

fn handle_io_events(state: &mut AppState, event: &Event) -> bool {
    match event {
        Event::BashOutput { command, output } => { state.add_system_msg(format!("$ {}\n{}", command, output)); state.view_mut().scroll = 0; state.messages_changed(); true }
        Event::FilesWritten { count, errors } => { state.add_system_msg(if errors.is_empty() { format!("Applied {} edit(s).", count) } else { format!("Applied {} edit(s). Errors: {}", count, errors.join(", ")) }); true }
        Event::EnvDetected { git_info, cwd_name } => { *state.git_info_mut() = git_info.clone(); *state.cwd_name_mut() = cwd_name.clone(); true }
        Event::FffSearchResult { request_id, entries, query: _, indexed: _ } => {
            // Only update if request_id matches current debounce (most recent request)
            if *request_id == state.fff_debounce() {
                *state.fff_file_results_mut() = entries.clone();
            }
            true
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventCategory { Input, Agent, Scroll, Control, ModelConfig, Dialog, Edit, System, Session, Command, LoginFlow, Permission, Other }

fn categorize(event: &Event) -> EventCategory {
    if is_permission_event(event) { return EventCategory::Permission; }
    if is_input_event(event) { return EventCategory::Input; }
    if is_agent_event(event) { return EventCategory::Agent; }
    if is_scroll_event(event) { return EventCategory::Scroll; }
    if is_control_event(event) { return EventCategory::Control; }
    if is_model_config_event(event) { return EventCategory::ModelConfig; }
    if is_dialog_category_event(event) { return EventCategory::Dialog; }
    if let Some(cat) = categorize_edit_system_session(event) { return cat; }
    if let Some(cat) = categorize_command_login(event) { return cat; }
    EventCategory::Other
}

fn is_permission_event(e: &Event) -> bool {
    matches!(e, Event::PermissionRequest { .. } | Event::PermissionResponse { .. } | Event::PermissionRequestDismissed)
}

fn is_scroll_event(e: &Event) -> bool {
    matches!(e, Event::Up | Event::Down)
}

fn categorize_edit_system_session(e: &Event) -> Option<EventCategory> {
    match e {
        Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit => Some(EventCategory::Edit),
        Event::SystemMessage { .. } | Event::TransientMessage { .. } | Event::TransientError { .. } | Event::ClearTransient | Event::ShowDiagnostics => Some(EventCategory::System),
        Event::ForkSession { .. } | Event::CloneSession | Event::ToggleSessionTree | Event::SessionTreeFilterCycle | Event::SessionTreeSelect { .. } => Some(EventCategory::Session),
        _ => None,
    }
}

fn categorize_command_login(e: &Event) -> Option<EventCategory> {
    match e {
        Event::RunLoadCommand { .. } | Event::RunSaveCommand { .. } | Event::RunDeleteCommand { .. } | Event::RunImportCommand { .. } | Event::RunExportCommand { .. } | Event::RunSkillCommand { .. } | Event::RunLoginCommand { .. } | Event::RunLogoutCommand { .. } | Event::RunNameCommand { .. } | Event::RunForkCommand { .. } | Event::RunCompactCommand { .. } | Event::RunPromptCommand { .. } | Event::RunThinkingCommand { .. } | Event::RunPaletteCommand { .. } => Some(EventCategory::Command),
        Event::Start | Event::SelectProvider { .. } | Event::SubmitKey { .. } | Event::ValidationFailed { .. } | Event::ModelsFetched { .. } | Event::ToggleModel { .. } | Event::Save | Event::Cancel => Some(EventCategory::LoginFlow),
        _ => None,
    }
}

fn is_input_event(event: &Event) -> bool {
    matches!(event, Event::Input(_) | Event::Backspace | Event::Newline | Event::Submit | Event::Escape | Event::CursorLeft | Event::CursorRight | Event::CursorStart | Event::CursorEnd | Event::DeleteWord | Event::DeleteToEnd | Event::DeleteToStart | Event::KillChar | Event::HistoryPrev | Event::HistoryNext | Event::Undo | Event::Redo | Event::CursorWordLeft | Event::CursorWordRight | Event::PageUp | Event::PageDown | Event::GoToTop | Event::GoToBottom | Event::Paste(_) | Event::PasteImage | Event::MouseClick { .. } | Event::MouseRelease { .. } | Event::MouseDrag { .. } | Event::MouseMove { .. } | Event::MouseScrollUp | Event::MouseScrollDown | Event::FocusGained | Event::FocusLost | Event::TerminalSize { .. })
}

fn is_agent_event(event: &Event) -> bool {
    matches!(event, Event::Thinking { .. } | Event::ThoughtDone { .. } | Event::ToolStart { .. } | Event::ToolEnd { .. } | Event::ResponseDelta { .. } | Event::Response { .. } | Event::TurnComplete { .. } | Event::Done { .. } | Event::Error { .. } | Event::TextStart { .. } | Event::TextEnd { .. } | Event::ThinkingStart { .. } | Event::ThinkingDelta { .. } | Event::ThinkingEnd { .. })
}

fn is_control_event(event: &Event) -> bool {
    matches!(event, Event::Quit | Event::ForceQuit | Event::Reset | Event::Abort | Event::ClearQueues | Event::FollowUp | Event::ToggleExpand | Event::Dequeue | Event::OpenExternalEditor | Event::ExternalEditorDone { .. } | Event::ShareSession | Event::Suspend | Event::ToggleVimMode | Event::CopyLastResponse | Event::OpenSessionList | Event::NewSession | Event::ResumeSession | Event::SelectSession { .. } | Event::StarSession { .. } | Event::RenameSession { .. } | Event::DeleteSession { .. })
}

fn is_model_config_event(event: &Event) -> bool {
    matches!(event, Event::SwitchModel { .. } | Event::SwitchTheme { .. } | Event::CycleModelNext | Event::CycleModelPrev | Event::ToggleScopedModelsDialog | Event::ScopedModelToggle { .. } | Event::ScopedModelEnableAll | Event::ScopedModelDisableAll | Event::ScopedModelToggleProvider { .. } | Event::ToggleSettingsDialog | Event::SettingsUp | Event::SettingsDown | Event::SettingsLeft | Event::SettingsRight | Event::SettingsSelect | Event::SettingsClose | Event::SettingsSwitchCategory { .. } | Event::CycleThinkingLevel | Event::SetThinkingLevel(_) | Event::ToggleReadOnly | Event::TrustProject | Event::UntrustProject | Event::ReloadAll | Event::KeybindingsReloaded)
}

fn is_dialog_category_event(event: &Event) -> bool {
    is_palette_selector_event(event) || is_path_form_event(event) || matches!(event, Event::ToggleWelcome | Event::DialogBack | Event::ProvidersDialog | Event::ProvidersSelectModel { .. } | Event::ProvidersDisconnect { .. } | Event::ProvidersAdd | Event::ProvidersEditModels { .. } | Event::CopyToClipboard(_) | Event::CopySelectedBlock | Event::CopyBlockMetadata | Event::AtFilePicker | Event::InsertAtRef(_))
}

fn is_palette_selector_event(event: &Event) -> bool {
    matches!(event, Event::ToggleCommandPalette | Event::PaletteFilter(_) | Event::PaletteBackspace | Event::PaletteUp | Event::PaletteDown | Event::PaletteSelect | Event::PaletteClose | Event::ToggleModelSelector | Event::ModelSelectorFilter(_) | Event::ModelSelectorBackspace | Event::ModelSelectorUp | Event::ModelSelectorDown | Event::ModelSelectorSelect | Event::ModelSelectorClose)
}

fn is_path_form_event(event: &Event) -> bool {
    matches!(event, Event::TogglePathCompletion | Event::PathCompletionUp | Event::PathCompletionDown | Event::PathCompletionSelect | Event::PathCompletionClose) || is_form_dialog_event(event)
}

fn is_form_dialog_event(event: &crate::Event) -> bool {
    matches!(event, crate::Event::CommandFormInput(_) | crate::Event::CommandFormBackspace | crate::Event::CommandFormUp | crate::Event::CommandFormDown | crate::Event::CommandFormSubmit | crate::Event::CommandFormClose)
}

fn dispatch_dialog_event(state: &mut AppState, event: crate::Event) {
    if is_toggle_dialog_event(&event) {
        super::dialog::dialog_toggle_event(state, event);
    } else if is_form_dialog_event(&event) {
        super::dialog::handle_form_dialog(state, event);
    } else if let crate::Event::InsertAtRef(path) = event {
        super::dialog::insert_at_ref(state, &path);
    } else if matches!(event, crate::Event::DialogBack) {
        handle_dialog_back_no_dialog(state);
    }
}

fn handle_dialog_back_no_dialog(state: &mut AppState) {
    if state.open_dialog().is_none() && state.config_mut().vim_mode {
        state.view_mut().vim_nav_mode = true;
        state.view_mut().selected_post = state.current_bottom_post_index();
        state.view_mut().dirty = true;
    }
}

pub(crate) fn is_dialog_event(event: &Event) -> bool {
    is_toggle_dialog_event(event) || is_form_dialog_event(event) || matches!(event, Event::InsertAtRef(_) | Event::DialogBack)
}

fn is_toggle_dialog_event(event: &crate::Event) -> bool {
    is_palette_selector_event(event) || is_path_form_event(event) || matches!(event, crate::Event::ToggleWelcome | crate::Event::ToggleSettingsDialog | crate::Event::ToggleModelSelector | crate::Event::AtFilePicker | crate::Event::ToggleVimMode | crate::Event::ProvidersDialog | crate::Event::ProvidersAdd | crate::Event::ProvidersEditModels { .. } | crate::Event::ProvidersSelectModel { .. } | crate::Event::ProvidersDisconnect { .. } | crate::Event::ToggleScopedModelsDialog | crate::Event::ScopedModelEnableAll | crate::Event::ScopedModelDisableAll)
}
