//! Central event dispatcher.

use crate::actors::turn::TurnMsg;
use crate::event::EventCategory;
use crate::model::AppState;
use crate::Event;

pub(crate) fn dispatch_event(state: &mut AppState, event: Event) {
    if try_handle_early_events(state, &event) {
        return;
    }
    match event.category() {
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
        EventCategory::IO => {
            let _ = handle_io_events(state, &event);
        }
        EventCategory::Persistence => {
            let _ = handle_persistence_events(state, &event);
        }
        EventCategory::Other | EventCategory::Unknown => {}
    }
}

fn try_handle_early_events(state: &mut AppState, event: &Event) -> bool {
    if let Event::MessageReplayed {
        id,
        role,
        content,
        timestamp,
        provider,
    } = event
    {
        state.replay_message(
            id.clone(),
            role.clone(),
            content.clone(),
            *timestamp,
            provider.clone(),
        );
        return true;
    }
    if let Event::SetPrompt { name } = event {
        state.input_mut().current_prompt = name.clone();
        return true;
    }
    handle_turn_events(state, event)
        || handle_persistence_events(state, event)
        || handle_session_store_events(state, event)
        || handle_io_events(state, event)
}

fn handle_turn_events(state: &mut AppState, event: &Event) -> bool {
    match event {
        Event::TurnAborted => {
            state.apply_turn_aborted();
            true
        }
        Event::QueueAborted { content } => {
            state.apply_queue_aborted(content.clone());
            true
        }
        Event::TurnStarted { .. } => {
            state.apply_turn_started();
            true
        }
        Event::TurnCompleted => {
            state.apply_turn_completed();
            true
        }
        Event::TurnErrored { .. } => {
            state.apply_turn_errored();
            true
        }
        Event::TokenStatsUpdated {
            tokens_in,
            tokens_out,
            speed_tps,
        } => {
            state.apply_token_stats(*tokens_in, *tokens_out, *speed_tps);
            true
        }
        Event::UserMessageSubmitted { id, content } => {
            state.apply_user_message_submitted(id.clone(), content.clone());
            true
        }
        Event::SteeringDelivered { content, id } => {
            state.apply_steering_delivered(content.clone(), id.clone());
            true
        }
        Event::FollowUpDelivered { content, id } => {
            state.apply_follow_up_delivered(content.clone(), id.clone());
            true
        }
        Event::MessageDequeued { content } => {
            state.apply_message_dequeued(content.clone());
            true
        }
        _ => false,
    }
}

/// Route agent events through TurnActor and handle facts synchronously.
fn handle_agent_event(state: &mut AppState, event: Event) {
    if let Some(handles) = state.actor_handles() {
        if let Some(turn_msg) = to_turn_msg(&event) {
            let _ = handles.turn.try_send(turn_msg);
        }
    }
    super::agent::agent_event(state, event);
}

fn to_turn_msg(event: &Event) -> Option<TurnMsg> {
    match event {
        Event::Thinking { id } => Some(TurnMsg::Thinking { id: id.clone() }),
        Event::ThoughtDone { id } => Some(TurnMsg::ThoughtDone { id: id.clone() }),
        Event::ToolStart { id, name, .. } => Some(TurnMsg::ToolStart {
            id: id.clone(),
            name: name.clone(),
        }),
        Event::ToolEnd {
            id,
            duration_secs,
            output,
        } => Some(TurnMsg::ToolEnd {
            id: id.clone(),
            duration_secs: *duration_secs,
            output: output.clone(),
        }),
        Event::ResponseDelta { id, content } => Some(TurnMsg::ResponseDelta {
            id: id.clone(),
            content: content.clone(),
        }),
        Event::TurnComplete { id, duration_secs } => Some(TurnMsg::TurnComplete {
            id: id.clone(),
            duration_secs: *duration_secs,
        }),
        Event::Done { id } => Some(TurnMsg::Done { id: id.clone() }),
        Event::Error { id, message } => Some(TurnMsg::Error {
            id: id.clone(),
            message: message.clone(),
        }),
        Event::TurnConstraintError { id, .. } => Some(TurnMsg::Error {
            id: id.clone(),
            message: "Tool constraint violation".to_string(),
        }),
        _ => None,
    }
}

fn handle_persistence_events(state: &mut AppState, event: &Event) -> bool {
    use crate::event::TransientLevel;
    match event {
        Event::TrustLoaded { decisions } => {
            state.set_trust_decisions(decisions.clone());
            true
        }
        Event::TrustChanged { path, decision } => {
            state.set_trust_decision(path.clone(), *decision);
            let new_read_only = !matches!(decision, crate::trust::TrustDecision::Trusted);
            state.config_mut().read_only = new_read_only;
            if matches!(decision, crate::trust::TrustDecision::Trusted) {
                state
                    .session_mut()
                    .messages
                    .retain(|m| m.id != "trust_welcome");
                state.messages_changed();
                state.notify(
                    format!("Project '{}' trusted. Read-only disabled.", &*path),
                    TransientLevel::Success,
                );
            } else {
                state.notify(
                    format!("Project '{}' untrusted. Read-only enabled.", &*path),
                    TransientLevel::Warning,
                );
            }
            true
        }
        Event::ReadOnlyChanged { enabled } => {
            state.config_mut().read_only = *enabled;
            true
        }
        Event::HistoryLoaded { entries } => {
            if let Some(handles) = state.actor_handles() {
                let _ = handles
                    .input
                    .send_message(crate::actors::InputMsg::HistoryLoaded {
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
        Event::SessionLoaded {
            name,
            events,
            metadata,
        } => {
            apply_session_loaded(state, name, events, metadata);
            true
        }
        Event::SessionSaved { name } => {
            state.notify(format!("Session '{}' saved.", name), TransientLevel::Info);
            true
        }
        Event::SessionDeleted { name } => {
            state.notify(format!("Session '{}' deleted.", name), TransientLevel::Info);
            true
        }
        Event::SessionImported { session } => {
            apply_session_imported(state, session);
            true
        }
        Event::SessionExported { path } => {
            state.notify(
                format!("Session exported to '{}'.", path),
                TransientLevel::Info,
            );
            true
        }
        Event::SessionList { sessions } => {
            apply_session_list(state, sessions);
            true
        }
        Event::SessionOperationFailed { operation, error } => {
            state.notify(
                format!("{} failed: {}", operation, error),
                TransientLevel::Error,
            );
            true
        }
        _ => false,
    }
}

fn apply_session_loaded(
    state: &mut AppState,
    name: &str,
    events: &[crate::event::DurableCoreEvent],
    metadata: &Option<Box<crate::session::SessionMetadata>>,
) {
    crate::session::replay::replay_events(state, events);
    if let Some(meta) = metadata {
        state.session_mut().session_display_name = Some(meta.display_name.clone());
        state.session_mut().session_created_at = meta.created_at;
        state.session_mut().session_updated_at = meta.updated_at;
    }
    state.configure_token_tracker();
    state.messages_changed();
    state.notify(
        format!("Session '{}' loaded.", name),
        crate::event::TransientLevel::Info,
    );
}

fn apply_session_imported(state: &mut AppState, session: &crate::session::Session) {
    state.restore_session(session);
    state.notify(
        format!("Session imported from '{}'.", session.name),
        crate::event::TransientLevel::Info,
    );
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
        Event::BashOutput { command, output } => {
            state.add_system_msg(format!("$ {}\n{}", command, output));
            state.view_mut().scroll = 0;
            state.messages_changed();
            true
        }
        Event::FilesWritten { count, errors } => {
            state.add_system_msg(if errors.is_empty() {
                format!("Applied {} edit(s).", count)
            } else {
                format!("Applied {} edit(s). Errors: {}", count, errors.join(", "))
            });
            true
        }
        Event::EnvDetected { git_info, cwd_name } => {
            *state.git_info_mut() = git_info.clone();
            *state.cwd_name_mut() = cwd_name.clone();
            true
        }
        Event::FffSearchResult {
            request_id,
            entries,
            ..
        } => {
            if *request_id == state.fff_debounce() {
                *state.fff_file_results_mut() = entries.clone();
                // Rebuild the file picker panel so new results appear immediately.
                // Safe even if dialog is closed — rebuild_file_picker is a no-op in that case.
                super::dialog::rebuild_file_picker(state);
            }
            true
        }
        _ => false,
    }
}

// ── Dialog routing helpers ─────────────────────────────────────────────────────
//
// These route within the Dialog category, not for top-level categorization.

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

/// Returns true if this event is a dialog toggle (opens/closes a dialog).
pub(crate) fn is_dialog_event(event: &Event) -> bool {
    is_toggle_dialog_event(event)
        || is_form_dialog_event(event)
        || matches!(event, Event::InsertAtRef(_) | Event::DialogBack)
}

fn is_toggle_dialog_event(event: &crate::Event) -> bool {
    is_palette_selector_event(event)
        || is_path_form_event(event)
        || matches!(
            event,
            crate::Event::ToggleWelcome
                | crate::Event::ToggleSettingsDialog
                | crate::Event::ToggleModelSelector
                | crate::Event::AtFilePicker
                | crate::Event::ToggleVimMode
                | crate::Event::ProvidersDialog
                | crate::Event::ProvidersAdd
                | crate::Event::ProvidersEditModels { .. }
                | crate::Event::ProvidersSelectModel { .. }
                | crate::Event::ProvidersDisconnect { .. }
                | crate::Event::ToggleScopedModelsDialog
                | crate::Event::ScopedModelEnableAll
                | crate::Event::ScopedModelDisableAll
        )
}

fn is_palette_selector_event(event: &crate::Event) -> bool {
    matches!(
        event,
        crate::Event::ToggleCommandPalette
            | crate::Event::PaletteFilter(_)
            | crate::Event::PaletteBackspace
            | crate::Event::PaletteUp
            | crate::Event::PaletteDown
            | crate::Event::PaletteSelect
            | crate::Event::PaletteClose
            | crate::Event::ToggleModelSelector
            | crate::Event::ModelSelectorFilter(_)
            | crate::Event::ModelSelectorBackspace
            | crate::Event::ModelSelectorUp
            | crate::Event::ModelSelectorDown
            | crate::Event::ModelSelectorSelect
            | crate::Event::ModelSelectorClose
    )
}

fn is_path_form_event(event: &crate::Event) -> bool {
    matches!(
        event,
        crate::Event::TogglePathCompletion
            | crate::Event::PathCompletionUp
            | crate::Event::PathCompletionDown
            | crate::Event::PathCompletionSelect
            | crate::Event::PathCompletionClose
    ) || is_form_dialog_event(event)
}

fn is_form_dialog_event(event: &crate::Event) -> bool {
    matches!(
        event,
        crate::Event::CommandFormInput(_)
            | crate::Event::CommandFormBackspace
            | crate::Event::CommandFormUp
            | crate::Event::CommandFormDown
            | crate::Event::CommandFormSubmit
            | crate::Event::CommandFormClose
    )
}
