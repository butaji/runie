//! Shared state helpers used by multiple update handlers.

use crate::actors::{ConfigMsg, TurnMsg};
use crate::event::TransientLevel;
use crate::model::{AppState, ChatMessage, Role};

mod model;

impl AppState {
    pub(crate) fn push_dialog_to_back_stack(&mut self, dialog: crate::commands::DialogState) {
        self.dialog_back_stack_mut().push(dialog);
    }
    pub fn peek_queue(&mut self) -> Option<&(String, String)> {
        self.agent_state_mut().request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.agent_state_mut().request_queue.pop_front()
    }
    pub(crate) fn set_transient(&mut self, content: String, level: TransientLevel) {
        *self.transient_message_mut() = Some(content);
        *self.transient_level_mut() = Some(level);
        *self.transient_until_mut() = match level {
            TransientLevel::Error => None,
            _ => Some(std::time::Instant::now() + std::time::Duration::from_secs(5)),
        };
        self.view_mut().dirty = true;
    }

    /// Show a warning notification in the hints line.
    pub(crate) fn warn(&mut self, msg: impl Into<String>) {
        self.set_transient(msg.into(), TransientLevel::Warning);
    }

    pub(crate) fn clear_transient(&mut self) {
        *self.transient_message_mut() = None;
        *self.transient_until_mut() = None;
        *self.transient_level_mut() = None;
        self.view_mut().dirty = true;
    }

    pub(crate) fn add_system_msg(&mut self, content: String) {
        self.session_mut().messages.push(ChatMessage {
            role: Role::System,
            timestamp: crate::update::now(),
            id: "system".to_owned(),
            parts: vec![runie_core::message::Part::Text { content }],
            ..Default::default()
        });
        self.messages_changed();
        *self.transient_until_mut() =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
    }

    /// Emit a transient notification in the hints line (not in the feed).
    pub(crate) fn notify(&mut self, content: String, level: TransientLevel) {
        self.set_transient(content, level);
    }
    /// Move TurnComplete to the end of messages and bump its timestamp.
    /// Called after every agent event to ensure TurnComplete remains last.
    /// Only moves the TurnComplete for the current turn (matching current_request_id
    /// or falling back to the last assistant message's id), so earlier turns'
    /// TurnComplete are not affected.
    pub(crate) fn ensure_turn_complete_last(&mut self) {
        let target_id = self
            .agent_state_mut()
            .current_request_id
            .clone()
            .or_else(|| {
                self.agent
                    .last_assistant_index
                    .and_then(|idx| self.session_mut().messages.get(idx).map(|m| m.id.clone()))
            });
        let Some(target_id) = target_id else {
            return;
        };
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == target_id)
        {
            let mut tc = self.session_mut().messages.remove(idx);
            tc.timestamp = crate::update::now();
            self.session_mut().messages.push(tc);
            self.messages_changed();
        }
    }

    // === View Helpers ===
    pub(crate) fn toggle_expand_all(&mut self) {
        self.view_mut().all_collapsed = !self.view_mut().all_collapsed;
        // Per-post expansions only make sense while globally collapsed.
        self.view_mut().expanded_posts.clear();
        self.messages_changed();
    }

    pub(crate) fn page_up(&mut self) {
        crate::update::input::scroll_event(self, Event::PageUp);
    }

    pub(crate) fn page_down(&mut self) {
        crate::update::input::scroll_event(self, Event::PageDown);
    }

    pub(crate) fn go_to_top(&mut self) {
        crate::update::input::scroll_event(self, Event::GoToTop);
    }

    pub(crate) fn go_to_bottom(&mut self) {
        crate::update::input::scroll_event(self, Event::GoToBottom);
    }

    // === Model / Config Helpers ===

    pub(crate) fn configure_token_tracker(&mut self) {
        let config = self.config();
        self.agent_state_mut().token_tracker =
            crate::tokens::token_tracker_for(&config.current_provider, &config.current_model);
    }

    pub(crate) fn switch_theme(&mut self, name: String) {
        // Validate theme name
        if !crate::theme_tokens::BUILTIN_THEMES.contains(&name.as_str()) {
            self.add_system_msg(format!(
                "Theme '{}' not found. Use /theme to list available themes.",
                name
            ));
            return;
        }
        if self.config_mut().theme_name == name {
            return;
        }
        self.config_mut().theme_name = name.clone();
        // Persist to config.toml via ConfigActor (fire-and-forget).
        // In tests without actor handles, the mutation is already applied above.
        if let Some(h) = self.actor_handles() {
            let name_clone = name.clone();
            let _ = h.config.try_send(ConfigMsg::SetTheme { name: name_clone });
        }
        self.add_system_msg(format!("Theme switched to '{}'", name));
    }

    pub(crate) fn toggle_read_only(&mut self) {
        self.config_mut().read_only = !self.config_mut().read_only;
        let status = if self.config_mut().read_only {
            "enabled"
        } else {
            "disabled"
        };
        self.notify(
            format!("Read-only mode {}", status),
            TransientLevel::Warning,
        );
    }

    pub(crate) fn stop_turn(&mut self) {
        // Route through TurnActor to maintain authoritative turn state.
        // Also send AgentMsg::Abort directly to the agent actor for cancellation.
        if let Some(h) = self.actor_handles() {
            let _ = h.turn.try_send(TurnMsg::AbortTurn);
            // Route abort directly to agent actor (not through event bus).
            let agent = h.agent.clone();
            tokio::spawn(async move {
                agent.abort().await;
            });
        } else {
            // Fallback for tests without actor handles
            self.apply_turn_aborted();
            // Also drain queue back to input (matches original behavior)
            let messages: Vec<_> = self
                .agent_state_mut()
                .message_queue
                .drain(..)
                .rev()
                .collect();
            for msg in messages {
                self.apply_queue_aborted(msg.content);
            }
        }
    }

    /// Handle TurnAborted fact — clear turn flags in AppState projection.
    pub(crate) fn apply_turn_aborted(&mut self) {
        let agent = self.agent_state_mut();
        agent.turn_active = false;
        agent.streaming = false;
        agent.inflight = 0;
        agent.current_request_id = None;
        agent.current_tool_name = None;
        agent.current_action = None;
        agent.turn_started_at = None;
        agent.thinking_started_at = None;
        agent.tool_started_at = None;
        self.view_mut().dirty = true;
    }

    /// Handle QueueAborted fact — restore aborted message content to input.
    pub(crate) fn apply_queue_aborted(&mut self, content: String) {
        let input = &mut self.input_mut().input;
        if !input.is_empty() {
            input.push('\n');
        }
        input.push_str(&content);
        self.input_mut().cursor_pos = input.len();
        self.view_mut().dirty = true;
    }
}

// ── Control Event Handler (merged from control.rs) ────────────────────────────

use crate::actors::PermissionMsg;
use crate::Event;

pub fn control_event(state: &mut AppState, event: Event) {
    match event {
        Event::Quit | Event::ForceQuit => handle_quit_event(state, event),
        Event::Reset => handle_reset(state),
        Event::Abort => handle_abort(state),
        Event::ClearQueues => handle_clear_queues(state),
        Event::ExternalEditorDone { content } => handle_editor_done(state, content),
        Event::ToggleExpand => state.toggle_expand_all(),
        Event::FollowUp => state.queue_follow_up(),
        Event::Dequeue => state.dequeue(),
        Event::ToggleVimMode => handle_toggle_vim_mode(state),
        Event::NewSession => handle_new_session(state),
        Event::ResumeSession | Event::OpenSessionList => {
            // Close welcome and open session tree
            crate::update::dialog::open_session_tree_dialog(state);
        }
        // intentionally ignored: these events are handled elsewhere
        Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => {}
        // intentionally ignored: other ControlEvent variants fall through
        _ => {}
    }
}

fn handle_toggle_vim_mode(state: &mut AppState) {
    let new_value = !state.config().vim_mode;
    state.config_mut().vim_mode = new_value;
    // Persist to config.toml via ConfigActor (fire-and-forget).
    // In tests without handles, the mutation is already applied above.
    if let Some(h) = state.actor_handles() {
        let _ = h
            .config
            .try_send(ConfigMsg::SetVimMode { enabled: new_value });
    }
    state.view_mut().cached_settings_valid = false;
    state.view_mut().dirty = true;
}

fn handle_new_session(state: &mut AppState) {
    // Abort in-flight turns via TurnActor (sends TurnMsg::AbortTurn).
    // This clears turn_active and the request/message queues in TurnActor.
    // UiActor's clear_turn_state(is_abort=true) runs after this and also
    // sends TurnMsg::ClearQueues for a clean queue reset.
    if let Some(h) = state.actor_handles() {
        let _ = h.turn.try_send(TurnMsg::AbortTurn);
        let _ = h.permission.try_send(PermissionMsg::DismissRequest);
    } else {
        // Fallback for tests without actor handles.
        state.agent_state_mut().turn_active = false;
        state.agent_state_mut().request_queue.clear();
        state.agent_state_mut().message_queue.clear();
    }
    // Reset session (messages, input, display name, dialogs, login flow).
    // Preserves config, actor_handles, git_info, cwd_name, trust_decisions.
    state.reset_session();
    // Restore timestamps and add system message.
    let now = crate::update::now();
    state.session_mut().session_created_at = now;
    state.session_mut().session_updated_at = now;
    state.messages_changed();
    state.add_system_msg(crate::ui_strings::session::NEW_SESSION_STARTED.into());
    // Configure token tracker for the current model.
    // Updates TurnState.token_tracker so AgentState.token_tracker derives correctly.
    if let Some(handles) = state.actor_handles() {
        let _ = handles.turn.try_send(TurnMsg::ConfigureTokenTracker {
            provider: state.config().current_provider.clone(),
            model: state.config().current_model.clone(),
        });
    } else {
        state.configure_token_tracker();
    }
    // Close welcome screen if open
    if matches!(
        state.open_dialog(),
        Some(crate::commands::DialogState::Welcome)
    ) {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
    }
    // Ready for user input — welcome is gone
    state.view_mut().dirty = true;
}

fn handle_clear_queues(state: &mut AppState) {
    // Route through TurnActor to maintain authoritative queue state.
    // Fallback clears queues directly in tests without actor handles.
    if let Some(h) = state.actor_handles() {
        let _ = h.turn.try_send(TurnMsg::ClearQueues);
    } else {
        state.agent_state_mut().request_queue.clear();
        state.agent_state_mut().message_queue.clear();
    }
}

fn handle_quit_event(state: &mut AppState, event: Event) {
    if matches!(event, Event::ForceQuit) {
        *state.should_quit_mut() = true;
        return;
    }
    // Quit/exit/:q must always close the app, even when a non-closable
    // onboarding dialog is open.
    if !state.input_mut().input.is_empty() {
        state.input_mut().input.clear();
        state.input_mut().cursor_pos = 0;
        state.input_mut().input_scroll = 0;
        state.input_mut().undo_stack.clear();
        state.input_mut().redo_stack.clear();
        state.view_mut().dirty = true;
    } else {
        *state.should_quit_mut() = true;
    }
}

fn handle_reset(state: &mut AppState) {
    state.reset_session();
    // Add confirmation message AFTER reset so it isn't cleared.
    state.add_system_msg(crate::ui_strings::session::STATE_CLEARED.into());
}

fn handle_abort(state: &mut AppState) {
    if state.completion_mut().path_suggestions.is_some() {
        state.path_completion_close();
        return;
    }
    if state.login_flow().is_some() {
        crate::login_flow::login_flow_cancel(state);
        // Safety net: keep onboarding open if the cancel path incorrectly closed it.
        if state.login_flow().is_some() && state.open_dialog().is_none() {
            tracing::warn!("onboarding dialog was incorrectly closed by Abort; reopening");
            crate::login_flow::rebuild_login_dialog(state);
        }
        return;
    }
    if state.open_dialog().is_some() && crate::update::dialog::root_closable(state) {
        // Close dialog when open
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        state.view_mut().dirty = true;
    } else if state.agent_state_mut().turn_active {
        state.stop_turn();
    } else {
        state.abort_queue();
    }
}

fn handle_editor_done(state: &mut AppState, content: String) {
    state.input_mut().input = content;
    state.input_mut().cursor_pos = state.input_mut().input.len();
    state.view_mut().dirty = true;
}

// ── System actions (merged from system_actions.rs) ───────────────────────────

impl AppState {
    pub(crate) fn show_diagnostics(&mut self) {
        let mut lines = vec!["Diagnostics:".to_owned()];
        let config_path = crate::config::config_path();
        lines.push(format!(
            "  Config: {}",
            if config_path.exists() {
                config_path.display().to_string()
            } else {
                "not found".to_owned()
            }
        ));
        let kb_path = crate::keybindings::default_keybindings_path();
        lines.push(format!(
            "  Keybindings: {}",
            if kb_path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                kb_path.unwrap().display().to_string()
            } else {
                "default".to_owned()
            }
        ));
        let config = self.config();
        lines.push(format!("  Theme: {}", config.theme_name));
        lines.push(format!(
            "  Provider: {}/{}",
            config.current_provider, config.current_model
        ));
        lines.push(format!("  Read-only: {}", config.read_only));
        lines.push(format!("  Scoped models: {}", config.scoped_models.len()));
        self.add_system_msg(lines.join("\n"));
    }
}

// ── Goal event handler ─────────────────────────────────────────────────────────

pub(super) fn handle_goal_event(state: &mut AppState, event: Event) {
    match event {
        Event::GoalCreate { objective } => {
            let goal = crate::model::GoalState::new(objective.clone(), None);
            *state.goal_state_mut() = Some(goal);
        }
        Event::GoalComplete { objective } => {
            if let Some(g) = state.goal_state_mut() {
                g.status = crate::model::GoalStatus::Completed;
            }
            state.add_system_msg(format!("Goal completed: {}", objective));
        }
        Event::GoalPause => {
            if let Some(g) = state.goal_state_mut() {
                if g.status == crate::model::GoalStatus::Active {
                    g.status = crate::model::GoalStatus::Paused;
                    state.notify("Goal paused.".into(), TransientLevel::Info);
                }
            }
        }
        Event::GoalResume => {
            if let Some(g) = state.goal_state_mut() {
                if g.status == crate::model::GoalStatus::Paused {
                    g.status = crate::model::GoalStatus::Active;
                    state.notify("Goal resumed.".into(), TransientLevel::Info);
                }
            }
        }
        Event::GoalCancel => {
            if let Some(g) = state.goal_state_mut() {
                let obj = g.objective.clone();
                *state.goal_state_mut() = None;
                state.add_system_msg(format!("Goal cancelled: {}", obj));
            }
        }
        _ => {}
    }
}

// ── System event dispatcher ──────────────────────────────────────────────────

pub(super) fn handle_system_event(state: &mut AppState, event: Event) {
    match event {
        Event::SystemMessage { content } => state.add_system_msg(content),
        Event::TransientMessage { content, level } => state.set_transient(content, level),
        Event::TransientError { content } => {
            state.set_transient(content, crate::event::TransientLevel::Error)
        }
        Event::ClearTransient => state.clear_transient(),
        Event::ShowDiagnostics => state.show_diagnostics(),
        Event::ToggleReadOnly => state.toggle_read_only(),
        // intentionally ignored: other SystemEvent variants fall through
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warn_sets_transient_warning() {
        let mut state = AppState::default();
        state.warn("test warning");
        assert_eq!(state.transient_message(), Some(&"test warning".to_string()));
        assert_eq!(state.transient_level(), Some(TransientLevel::Warning));
    }
}
