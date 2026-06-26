//! Submit, command dispatch, and history management.
//!
//! Submitting content emits `InputMsg::Clear` and `SessionMsg::AppendHistory`.
//! History navigation emits `InputMsg::HistoryPrev/Next`.

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
        let Some(content) = self.take_submit_content() else {
            return;
        };

        {
            let agent = self.agent_state_mut();
            agent.tokens_in += agent.token_tracker.estimate_input(&content);
        }

        if self.try_handle_bang_command(&content).is_some() {
            return;
        }

        // Emit history append through SessionActor.
        try_append_history(self, content.clone());
        // Direct mutation for tests (when InputActor is not spawned)
        self.push_to_input_history(&content);
        self.dispatch_submit_content(content);
    }

    fn try_handle_bang_command(&mut self, content: &str) -> Option<()> {
        let stripped = content.strip_prefix('!')?;
        let command = stripped.trim().to_owned();
        if !command.is_empty() {
            self.run_bash_command(&command);
        }
        Some(())
    }

    fn push_to_input_history(&mut self, content: &str) {
        let history = &mut self.input_mut().input_history;
        if history.is_empty() || history.last() != Some(&content.to_string()) {
            history.push(content.to_string());
        }
    }

    fn take_submit_content(&mut self) -> Option<String> {
        if self.input().input.is_empty() {
            self.input_mut().input_flash = 3;
            return None;
        }
        // Clear input through InputActor.
        try_send_input(self, crate::actors::InputMsg::Clear);
        // Direct mutation for tests (when InputActor is not spawned)
        let content = self.input().input.trim().to_owned();
        self.input_mut().input.clear();
        self.input_mut().cursor_pos = 0;
        self.input_mut().history_pos = None;
        self.input_mut().undo_stack.clear();
        self.input_mut().redo_stack.clear();
        self.input_mut().input_scroll = 0;
        if crate::update::input::is_quit_command(&content) {
            *self.should_quit_mut() = true;
            return None;
        }
        if content.is_empty() && self.session().image_attachments.is_empty() {
            return None;
        }
        Some(self.build_content_with_attachments(content))
    }

    fn build_content_with_attachments(&mut self, content: String) -> String {
        if self.session().image_attachments.is_empty() {
            return content;
        }
        let mut full = content;
        for uri in std::mem::take(&mut self.session_mut().image_attachments) {
            if !full.is_empty() {
                full.push('\n');
            }
            full.push_str("![image](");
            full.push_str(&uri);
            full.push(')');
        }
        full
    }

    fn dispatch_submit_content(&mut self, content: String) {
        if let Some(result) = self.handle_slash(&content) {
            self.apply_command_result(result);
            self.view_mut().scroll = 0;
            self.view_mut().dirty = true;
            return;
        }
        if self.agent_state().turn_active {
            self.agent_state_mut()
                .message_queue
                .push(crate::model::QueuedMessage {
                    content,
                    kind: crate::model::QueuedMessageKind::Steering,
                });
            self.view_mut().scroll = 0;
            self.view_mut().dirty = true;
            return;
        }
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
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
        self.view_mut().scroll = 0;
        self.messages_changed();
    }

    fn apply_command_result(&mut self, result: crate::commands::CommandResult) {
        use crate::commands::DialogType;
        match result {
            crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
            crate::commands::CommandResult::Warning(msg) => {
                self.notify(msg, crate::event::TransientLevel::Warning)
            }
            crate::commands::CommandResult::Event(evt) => self.update(evt),
            crate::commands::CommandResult::OpenDialog(d) => match d {
                DialogType::CommandPalette => crate::update::dialog::open_command_palette(self),
                DialogType::ModelSelector => crate::update::dialog::open_model_selector(self),
                DialogType::Settings => crate::update::dialog::open_settings_dialog(self),
                DialogType::ScopedModels => crate::update::dialog::open_scoped_models_dialog(self),
            },
            crate::commands::CommandResult::OpenPanelStack(stack) => {
                *self.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(*stack));
                self.view_mut().dirty = true;
            }
            crate::commands::CommandResult::None => {}
        }
    }

    fn run_bash_command(&mut self, command: &str) {
        // Extract handles before async work to avoid borrow conflicts.
        let handles = self.actor_handles().cloned();
        let can_spawn = handles.as_ref().is_some() && tokio::runtime::Handle::try_current().is_ok();

        if can_spawn {
            let command = command.to_owned();
            let handles = handles.unwrap();
            tokio::spawn(async move {
                handles.run_bash(command).await;
            });
            return;
        }
        let result = crate::update::tools::execute_bash(command);
        let output_msg = format!("$ {}\n{}", command, result);
        self.add_system_msg(output_msg);
        self.view_mut().scroll = 0;
        self.messages_changed();
    }

    pub(crate) fn history_prev(&mut self) {
        let has_history = !self.input().input_history.is_empty();
        if has_history {
            try_send_input(self, crate::actors::InputMsg::HistoryPrev);
            // Direct mutation for tests (when InputActor is not spawned)
            let pos = match self.input().history_pos {
                Some(p) if p > 0 => p - 1,
                Some(p) => p,
                None => self.input().input_history.len() - 1,
            };
            self.input_mut().history_pos = Some(pos);
            self.input_mut().input = self.input().input_history[pos].clone();
            self.input_mut().cursor_pos = self.input().input.len();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn history_next(&mut self) {
        let history_pos = self.input().history_pos;
        if history_pos.is_some() {
            try_send_input(self, crate::actors::InputMsg::HistoryNext);
            // Direct mutation for tests (when InputActor is not spawned)
            let pos = history_pos.unwrap() + 1;
            if pos >= self.input().input_history.len() {
                self.input_mut().history_pos = None;
                self.input_mut().input.clear();
                self.input_mut().cursor_pos = 0;
            } else {
                self.input_mut().history_pos = Some(pos);
                self.input_mut().input = self.input().input_history[pos].clone();
                self.input_mut().cursor_pos = self.input().input.len();
            }
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }
}

/// Fire-and-forget send to InputActor.
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(ref handles) = state.actor_handles() {
        handles.try_send_input(msg);
    }
}

/// Emit `SessionMsg::AppendHistory` to SessionActor (fire-and-forget).
fn try_append_history(state: &mut AppState, entry: String) {
    if let Some(ref handles) = state.actor_handles() {
        handles.try_send_append_history(entry);
    }
}
