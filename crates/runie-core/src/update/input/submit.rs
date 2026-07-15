//! Submit, command dispatch, and history management.
//!
//! Submitting content emits `InputMsg::Clear` and `SessionMsg::AppendHistory`.
//! History navigation emits `InputMsg::HistoryPrev/Next`.
//! Input state mutations go through `InputActor`; projection updates via
//! `Event::InputChanged`.

use crate::actors::turn::messages::MessageSource;
use crate::actors::{IoMsg, SessionMsg, TurnMsg};
use crate::message::{now, ChatMessage, Role};
use crate::model::AppState;

impl AppState {
    pub(crate) fn submit(&mut self) {
        if self.completion().at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.completion().path_suggestions.is_some() {
            self.path_completion_select();
            return;
        }
        self.accept_ghost();
        if self.open_dialog().is_some() {
            return;
        }

        if let Some(content) = self.take_submit_text() {
            if crate::update::input::is_quit_command(&content) {
                *self.should_quit_mut() = true;
                return;
            }
            if content.is_empty() && self.session().image_attachments.is_empty() {
                return;
            }
            self.estimate_and_add_tokens(&content);
            if self.try_handle_bang_command(&content).is_some() {
                return;
            }
            try_append_history(self, content.clone());
            // Also update input_history in AppState for synchronous tests.
            self.push_to_input_history(&content);
            self.dispatch_submit_content(content);
        }
    }

    /// Extract the submit text.
    /// Returns None if input is empty (and sets flash).
    /// Clears the input field synchronously in tests (via InputMsg::Clear) and
    /// via UiActor/InputActor in production.
    fn take_submit_text(&mut self) -> Option<String> {
        let input = self.input();
        if input.input.is_empty() {
            self.input_mut().input_flash = 3;
            return None;
        }
        let content = input.input.trim().to_owned();
        // In test mode (no actor handles), try_send_input applies synchronously.
        // In production, this sends to InputActor which clears and replies with InputChanged.
        try_send_input(self, crate::actors::InputMsg::Clear);
        Some(content)
    }

    /// Push content to input history. Called by both `submit()` (test path) and
    /// `UiActor::dispatch_submit_content` (production path) to keep AppState's
    /// `input_history` in sync with the session history sent to SessionActor.
    pub(crate) fn push_to_input_history(&mut self, content: &str) {
        let history = &mut self.input_mut().input_history;
        if history.is_empty() || history.last() != Some(&content.to_string()) {
            history.push(content.to_string());
        }
    }

    fn estimate_and_add_tokens(&mut self, content: &str) {
        let tokens = self.agent_state().token_tracker.estimate_input(content);
        self.agent_state_mut().tokens_in += tokens;
    }

    fn try_handle_bang_command(&mut self, content: &str) -> Option<()> {
        let stripped = content.strip_prefix('!')?;
        let command = stripped.trim().to_owned();
        if !command.is_empty() {
            self.run_bash_command(&command);
        }
        Some(())
    }

    fn dispatch_submit_content(&mut self, content: String) {
        if let Some(result) = self.handle_slash(&content) {
            self.apply_command_result(result);
            self.view_mut().scroll = 0;
            self.view_mut().dirty = true;
            return;
        }
        if self.agent_state().turn_active {
            self.queue_steering_message(content);
            return;
        }
        self.submit_user_message(content);
    }

    fn queue_steering_message(&mut self, content: String) {
        // Route through TurnActor to maintain authoritative queue state.
        // Fallback applies synchronously in tests without actor handles.
        if let Some(h) = self.actor_handles() {
            let _ = h.turn.try_send(TurnMsg::QueueSteering { content });
        } else {
            // Test mode: use projection method.
            self.apply_queue_steering_added(String::new(), content);
        }
        self.view_mut().scroll = 0;
        self.view_mut().dirty = true;
    }

    pub fn submit_user_message(&mut self, mut content: String) {
        // Append image attachments.
        if !self.session().image_attachments.is_empty() {
            for uri in std::mem::take(&mut self.session_mut().image_attachments) {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str("![image](");
                content.push_str(&uri);
                content.push(')');
            }
        }
        // Route through TurnActor to maintain authoritative queue state.
        // Fallback applies synchronously in tests without actor handles.
        let handles = self.actor_handles().cloned();
        if let Some(h) = handles {
            let id = self.next_id();
            let _ = h.turn.try_send(TurnMsg::SubmitUserMessage {
                content,
                id,
                source: MessageSource::Fresh,
            });
        } else {
            self.apply_user_message_sync(content);
        }
        self.view_mut().scroll = 0;
        self.messages_changed();
    }

    /// Apply user message synchronously (for tests without actor handles).
    fn apply_user_message_sync(&mut self, content: String) {
        let id = self.next_id();
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![runie_core::message::Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        // Update AgentState projection directly.
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
    }

    pub fn apply_command_result(&mut self, result: crate::commands::CommandResult) {
        use crate::commands::DialogType;
        match result {
            crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
            crate::commands::CommandResult::Warning(msg) => {
                self.notify(msg, crate::event::TransientLevel::Warning)
            }
            crate::commands::CommandResult::Event(evt) => {
                self.apply_command_event(evt);
            }
            crate::commands::CommandResult::Events(evts) => {
                for evt in evts {
                    self.apply_command_event(evt);
                }
            }
            crate::commands::CommandResult::OpenDialog(d) => match d {
                DialogType::CommandPalette => crate::update::dialog::open_command_palette(self),
                DialogType::ModelSelector => crate::update::dialog::open_model_selector(self),
                DialogType::ModeSelector => crate::update::dialog::open_mode_selector(self),
                DialogType::Settings => crate::update::dialog::open_settings_dialog(self),
                DialogType::ScopedModels => crate::update::dialog::open_scoped_models_dialog(self),
                DialogType::ThemeSelector => crate::update::dialog::open_theme_selector(self),
                DialogType::McpServers => crate::update::dialog::open_mcp_servers_dialog(self),
                DialogType::Skills => crate::update::dialog::open_skills_dialog(self),
            },
            crate::commands::CommandResult::OpenPanelStack(stack) => {
                *self.open_dialog_mut() = Some(crate::commands::DialogState::Active {
                    kind: crate::commands::DialogKind::Generic,
                    panels: (*stack).clone(),
                });
                self.view_mut().dirty = true;
            }
            crate::commands::CommandResult::None => {}
        }
    }

    /// Process a command-result event: apply state change AND add confirmation message.
    /// This bridges the old "return Message" pattern with the new "return Event" pattern
    /// while maintaining backward-compatible UX for command confirmations.
    fn apply_command_event(&mut self, evt: crate::Event) {
        // Add confirmation system message for command-specific events.
        // Transient notifications (via notify) are added by the update handler.
        match &evt {
            crate::Event::SwitchModel {
                provider, model, ..
            } => {
                self.add_system_msg(format!("Switched to {}/{}", provider, model));
            }
            crate::Event::SetThinkingLevel(level) => {
                use crate::ui_strings::model as m;
                self.add_system_msg(m::thinking_level(level.as_str()));
            }
            crate::Event::NewSession => {
                self.add_system_msg(crate::ui_strings::session::NEW_SESSION_STARTED.into());
            }
            _ => {}
        }
        self.update(evt);
    }

    fn run_bash_command(&mut self, command: &str) {
        let handles = self.actor_handles().cloned();

        if let Some(ref h) = handles {
            // Production mode: send to IoActor (non-blocking)
            let command = command.to_owned();
            let _ = h.io.try_send(IoMsg::RunBash {
                command,
                shell: true,
            });
            return;
        }
        // Test-only fallback: no actor handles, so we must run synchronously.
        // This is acceptable because test handlers are synchronous and must block to
        // produce a result. Production always has IoActor handles.
        use crate::shell::run_bash_sync;
        let cwd = std::env::current_dir().unwrap_or_default();
        let result = run_bash_sync(command, &cwd, &std::collections::HashMap::new(), true).output;
        let output_msg = format!("$ {}\n{}", command, result);
        self.add_system_msg(output_msg);
        self.view_mut().scroll = 0;
        self.messages_changed();
    }

    pub(crate) fn history_prev(&mut self) {
        if !self.input().input_history.is_empty() {
            try_send_input(self, crate::actors::InputMsg::HistoryPrev);
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn history_next(&mut self) {
        if self.input().history_pos.is_some() {
            try_send_input(self, crate::actors::InputMsg::HistoryNext);
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }
}

/// Fire-and-forget send to InputActor.
/// In test mode (no actor handles), applies the mutation synchronously so that
/// synchronous tests can assert on the updated state without awaiting the actor.
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(handles) = state.actor_handles() {
        let _ = handles.input.send_message(msg);
    } else {
        // Test mode: apply synchronously to AppState projection.
        msg.apply_to(state.input_mut());
    }
}

/// Emit `SessionMsg::AppendHistory` to SessionActor (fire-and-forget).
fn try_append_history(state: &mut AppState, entry: String) {
    if let Some(handles) = state.actor_handles() {
        let _ = handles
            .session
            .try_send(SessionMsg::AppendHistory { entry });
    }
}
