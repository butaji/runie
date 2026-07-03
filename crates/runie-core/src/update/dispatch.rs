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
        EventCategory::PlanMode => plan_mode_event(state, event),
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
            // Check if compaction should be triggered based on token ratio.
            if let Some(ctx) = state.current_model_context_window() {
                use crate::session::store::COMPACT_TOKEN_RATIO;
                let threshold = (ctx as f64 * COMPACT_TOKEN_RATIO) as usize;
                if *tokens_in > threshold {
                    dispatch_event(
                        state,
                        Event::CompactionTriggered {
                            ratio: COMPACT_TOKEN_RATIO,
                            tokens_in: *tokens_in,
                            context_window: ctx,
                        },
                    );
                }
            }
            true
        }
        Event::CompactionTriggered { tokens_in: _, context_window, .. } => {
            // Compaction keeps roughly COMPACT_TOKEN_RATIO of the context window.
            use crate::session::store::COMPACT_TOKEN_RATIO;
            let keep = (*context_window as f64 * COMPACT_TOKEN_RATIO) as usize;
            let _ = state.compact(keep);
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
        // Agent/streaming events emitted by TurnActor — use existing projection handlers.
        Event::Thinking { id } => {
            state.set_thinking(id.clone());
            true
        }
        Event::ThoughtDone { id } => {
            state.add_thought(id.clone());
            true
        }
        Event::ToolStart { id, name, .. } => {
            state.start_tool(id.clone(), name.clone());
            true
        }
        Event::ToolEnd { id: _, duration_secs, output, .. } => {
            state.end_tool(*duration_secs, output.clone());
            true
        }
        Event::ResponseDelta { id, content } => {
            state.handle_llm_event(Event::ResponseDelta { id: id.clone(), content: content.clone() });
            true
        }
        Event::TurnComplete { id, duration_secs } => {
            state.complete_turn(id.clone(), *duration_secs);
            true
        }
        Event::Done { id } => {
            state.finish_turn(id.clone());
            true
        }
        Event::Error { id, message } => {
            state.add_error(id.clone(), message.clone());
            true
        }
        Event::StreamStarted { id: _ } => {
            // StreamStarted is informational; streaming state is managed by other handlers.
            // Just set the streaming flag if not already.
            if !state.turn_state.streaming {
                state.turn_state_mut().streaming = true;
                *state.agent_state_mut() = crate::model::AgentState::from(&state.turn_state);
            }
            true
        }
        Event::QueuesCleared => {
            // QueuesCleared is emitted by TurnActor but AppState projections
            // are synced via the turn_state authoritative state.
            // Just sync from turn_state to agent_state.
            *state.agent_state_mut() = crate::model::AgentState::from(&state.turn_state);
            true
        }
        _ => false,
    }
}

/// Route agent events through TurnActor and apply projections.
///
/// TurnActor is the sole source of truth for turn state. Events are sent to
/// TurnActor, which emits facts that update AppState through handle_turn_events.
///
/// In production: TurnActor applies events and emits facts → idempotent guards
/// prevent double application in handle_turn_events.
///
/// In tests: No TurnActor runs, so we apply projections directly via agent_event.
/// The idempotent guards in projection handlers prevent issues from dual calls.
fn handle_agent_event(state: &mut AppState, event: Event) {
    if let Some(handles) = state.actor_handles() {
        if let Some(turn_msg) = to_turn_msg(&event) {
            let _ = handles.turn.try_send(turn_msg);
        }
    }
    // Apply projection via agent_event. This is idempotent, so even if TurnActor
    // also emits the event (which goes through handle_turn_events), the second
    // application is a no-op.
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
            ..
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
                    format!("Project '{}' trusted. Read-only disabled.", path),
                    TransientLevel::Success,
                );
            } else {
                state.notify(
                    format!("Project '{}' untrusted. Read-only enabled.", path),
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
    if let Some(ref meta) = metadata {
        state.restore_session_metadata(meta);
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
            state.set_git_info(git_info.clone());
            state.set_cwd_name(cwd_name.clone());
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

/// Handle plan mode events — enable/disable plan mode in ViewState.
fn plan_mode_event(state: &mut AppState, event: crate::Event) {
    match event {
        crate::Event::PlanModeEnabled { content } => {
            state.view_mut().plan_mode = true;
            let content_clone = content.clone();
            state.view_mut().active_plan_content = content_clone;
            state.view_mut().dirty = true;

            // Save plan to disk and store the plan ID
            if let Some(plans_dir) = crate::session::plan_persistence::default_plans_dir() {
                let session_id = state
                    .session()
                    .session_display_name
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                if let Ok(Some(plan_id)) =
                    crate::session::plan_persistence::save_plan(&plans_dir, &session_id, &content)
                {
                    state.view_mut().active_plan_id = Some(plan_id);
                    tracing::debug!("Saved plan for session {}", session_id);
                }
            }

            state.add_system_msg("Plan mode enabled. Write tools are blocked until plan is approved.".to_string());
        }
        crate::Event::PlanModeDisabled => {
            state.view_mut().plan_mode = false;
            state.view_mut().active_plan_content.clear();
            state.view_mut().active_plan_id = None;
            state.view_mut().dirty = true;
            state.add_system_msg("Plan mode disabled.".to_string());
        }
        _ => {}
    }
}
