//! Submit, command dispatch, and history management.

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

        if let Some(stripped) = content.strip_prefix('!') {
            let command = stripped.trim().to_owned();
            if !command.is_empty() {
                self.run_bash_command(&command);
            }
            return;
        }

        self.add_to_input_history(content.clone());
        self.dispatch_submit_content(content);
    }

    fn take_submit_content(&mut self) -> Option<String> {
        if self.input().input.is_empty() {
            self.input_mut().input_flash = 3;
            return None;
        }
        let content = {
            let input = self.input_mut();
            std::mem::take(&mut input.input).trim().to_owned()
        };
        {
            let input = self.input_mut();
            input.cursor_pos = 0;
            input.input_scroll = 0;
            input.history_pos = None;
            input.undo_stack.clear();
            input.redo_stack.clear();
        }
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
        let input = self.input();
        if input.input_history.is_empty() {
            self.input_mut().input_flash = 3;
            return;
        }
        let pos = match input.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => input.input_history.len() - 1,
        };
        {
            let input = self.input_mut();
            input.history_pos = Some(pos);
            input.input = input.input_history[pos].clone();
            input.cursor_pos = input.input.len();
        }
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn history_next(&mut self) {
        let input = self.input();
        let pos = match input.history_pos {
            Some(p) => p + 1,
            None => {
                self.input_mut().input_flash = 3;
                return;
            }
        };
        if pos >= input.input_history.len() {
            let input = self.input_mut();
            input.history_pos = None;
            input.input.clear();
            input.cursor_pos = 0;
        } else {
            let history_entry = input.input_history[pos].clone();
            let input = self.input_mut();
            input.history_pos = Some(pos);
            input.input = history_entry;
            input.cursor_pos = input.input.len();
        }
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }
}
