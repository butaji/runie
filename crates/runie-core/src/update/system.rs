//! Shared state helpers used by multiple update handlers.

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
            id: "system".to_string(),
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
        if self.config_mut().theme_name == name {
            return;
        }
        self.config_mut().theme_name = name.clone();
        self.notify(
            format!("Theme switched to '{}'", name),
            TransientLevel::Success,
        );
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

    pub(crate) fn apply_trust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        self.config_mut().read_only = false;
        self.session_mut()
            .messages
            .retain(|m| m.id != "trust_welcome");
        self.messages_changed();
        self.notify(
            format!("Project '{}' trusted. Read-only disabled.", cwd.display()),
            TransientLevel::Success,
        );
    }

    pub(crate) fn apply_untrust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        self.config_mut().read_only = true;
        self.notify(
            format!("Project '{}' untrusted. Read-only enabled.", cwd.display()),
            TransientLevel::Warning,
        );
    }

    pub(crate) fn stop_turn(&mut self) {
        // Stop the current turn and abort any queued messages
        self.agent_state_mut().turn_active = false;
        self.agent_state_mut().current_request_id = None;
        self.agent_state_mut().streaming = false;
        self.agent_state_mut().current_tool_name = None;
        self.agent_state_mut().current_action = None;
        self.agent_state_mut().inflight = 0;
        self.agent_state_mut().turn_started_at = None;
        self.agent_state_mut().thinking_started_at = None;
        self.agent_state_mut().tool_started_at = None;
        // Drain the queue back to input
        let messages: Vec<_> = self
            .agent_state_mut()
            .message_queue
            .drain(..)
            .rev()
            .collect();
        let input = &mut self.input_mut().input;
        for msg in messages {
            if !input.is_empty() {
                input.push('\n');
            }
            input.push_str(&msg.content);
        }
        self.input_mut().cursor_pos = self.input_mut().input.len();
        self.view_mut().dirty = true;
    }
}

/// Apply the trust decision for the given directory after it has been loaded.
pub fn apply_initial_trust(state: &mut AppState, cwd: &std::path::Path) {
    match state.trust_decisions().get(cwd).copied() {
        Some(crate::trust::TrustDecision::Untrusted) => {
            state.config_mut().read_only = true;
        }
        Some(crate::trust::TrustDecision::Trusted) => {
            state.config_mut().read_only = false;
        }
        None => {
            state.config_mut().read_only = false;
            state.session_mut().messages.push(ChatMessage {
                role: Role::System,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                id: "trust_welcome".to_string(),
                parts: vec![runie_core::message::Part::Text {
                    content: format!(
                        "Welcome to runie in {}.\n\nThis project is not yet trusted. \
                    Run /trust to enable write tools, or /untrust to enforce read-only mode.",
                        cwd.display()
                    ),
                }],
                ..Default::default()
            });
            state.messages_changed();
        }
    }
}

// ── Control Event Handler (merged from control.rs) ────────────────────────────

use crate::Event;

pub fn control_event(state: &mut AppState, event: Event) {
    match event {
        Event::Quit | Event::ForceQuit => handle_quit_event(state, event),
        Event::Reset => handle_reset(state),
        Event::Abort => handle_abort(state),
        Event::ExternalEditorDone { content } => handle_editor_done(state, content),
        Event::ToggleExpand => state.toggle_expand_all(),
        Event::FollowUp => state.queue_follow_up(),
        Event::Dequeue => state.dequeue(),
        Event::ToggleVimMode => {
            state.config_mut().vim_mode = !state.config_mut().vim_mode;
            state.view_mut().cached_settings_valid = false;
            state.view_mut().dirty = true;
        }
        Event::NewSession => {
            // Close welcome screen if open
            if matches!(
                state.open_dialog,
                Some(crate::commands::DialogState::Welcome)
            ) {
                *state.open_dialog_mut() = None;
                state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            }
            // Ready for user input — welcome is gone
            state.view_mut().dirty = true;
        }
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

fn handle_quit_event(state: &mut AppState, event: Event) {
    if matches!(event, Event::ForceQuit) {
        *state.should_quit_mut() = true;
        return;
    }
    if !crate::update::dialog::root_closable(state) {
        return;
    }
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
}

fn handle_abort(state: &mut AppState) {
    if state.completion_mut().path_suggestions.is_some() {
        state.path_completion_close();
        return;
    }
    if state.login_flow().is_some() {
        crate::login_flow::login_flow_cancel(state);
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
        let mut lines = vec!["Diagnostics:".to_string()];
        let config_path = crate::config::config_path();
        lines.push(format!(
            "  Config: {}",
            if config_path.exists() {
                config_path.display().to_string()
            } else {
                "not found".to_string()
            }
        ));
        let kb_path = crate::keybindings::default_keybindings_path();
        lines.push(format!(
            "  Keybindings: {}",
            if kb_path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                kb_path.unwrap().display().to_string()
            } else {
                "default".to_string()
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
        Event::TrustProject => state.apply_trust_project(),
        Event::UntrustProject => state.apply_untrust_project(),
        // intentionally ignored: other SystemEvent variants fall through
        _ => {}
    }
}

#[cfg(test)]
mod tests;
