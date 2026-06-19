//! Text editing (merged from input_text.rs).

use crate::message::{now, ChatMessage, Role};
use crate::model::AppState;

impl AppState {
    pub(crate) fn hint_text(&self) -> String {
        let mut parts = vec!["ctrl+o expand/collapse".to_string()];
        parts.extend(self.mode_hints());
        parts.push("ctrl+c quit".to_string());
        parts.join(" · ")
    }

    fn mode_hints(&self) -> Vec<String> {
        if self.open_dialog.is_some() {
            return crate::update::input::modal_hints();
        }
        if self.view.vim_nav_mode {
            return crate::update::input::vim_nav_hints();
        }
        if self.completion.at_suggestions.is_some() {
            return crate::update::input::at_suggestion_hints();
        }
        if self.agent.turn_active {
            return self.with_team_mode(self.active_turn_hints());
        }
        if !self.input.input.is_empty() {
            return self.with_team_mode(crate::update::input::input_active_hints());
        }
        if self.config.vim_mode {
            return self.with_team_mode(vec!["esc nav".to_string()]);
        }
        self.with_team_mode(crate::update::input::empty_input_hints())
    }

    fn with_team_mode(&self, mut hints: Vec<String>) -> Vec<String> {
        if self.config.execution_mode.uses_orchestrator() {
            hints.extend(crate::update::input::team_mode_hints());
        }
        hints
    }

    fn active_turn_hints(&self) -> Vec<String> {
        let esc = if self.config.vim_mode {
            "esc abort·nav"
        } else {
            "esc abort"
        };
        vec![
            "enter steer".to_string(),
            "alt+enter follow-up".to_string(),
            esc.to_string(),
        ]
    }

    pub(crate) fn insert_char(&mut self, c: char) {
        self.push_undo();
        if self.input.cursor_pos == self.input.input.len() {
            self.input.input.push(c);
        } else {
            self.input.input.insert(self.input.cursor_pos, c);
        }
        self.input.cursor_pos += c.len_utf8();
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn delete_before_cursor(&mut self) {
        if self.input.cursor_pos > 0 {
            self.delete_before_cursor_with_content();
        } else if self.input.cursor_pos == 0 && !self.input.input.is_empty() {
            self.delete_leading_newline();
        } else {
            self.input.input_flash = 3;
        }
    }

    fn delete_before_cursor_with_content(&mut self) {
        let char_before_cursor = self.input.input[..self.input.cursor_pos].chars().last();
        if char_before_cursor == Some('\n') {
            self.remove_newline_before_cursor();
        } else {
            self.remove_grapheme_before_cursor();
        }
    }

    fn remove_newline_before_cursor(&mut self) {
        self.push_undo();
        let new_pos = self.input.cursor_pos - 1;
        self.input.input.remove(self.input.cursor_pos - 1);
        self.input.cursor_pos = new_pos;
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    fn remove_grapheme_before_cursor(&mut self) {
        self.push_undo();
        let new_pos =
            crate::update::input::prev_grapheme_boundary(&self.input.input, self.input.cursor_pos);
        self.input.input.drain(new_pos..self.input.cursor_pos);
        self.input.cursor_pos = new_pos;
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    fn delete_leading_newline(&mut self) {
        if self.input.input.starts_with('\n') {
            self.push_undo();
            self.input.input.remove(0);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn delete_word(&mut self) {
        if self.input.cursor_pos == 0 {
            self.input.input_flash = 3;
            return;
        }
        self.push_undo();
        let start =
            crate::update::input::find_word_boundary_left(&self.input.input, self.input.cursor_pos);
        self.input.input.drain(start..self.input.cursor_pos);
        self.input.cursor_pos = start;
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn delete_to_end(&mut self) {
        if self.input.cursor_pos < self.input.input.len() {
            self.push_undo();
            self.input.input.truncate(self.input.cursor_pos);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn delete_to_start(&mut self) {
        if self.input.cursor_pos > 0 {
            self.push_undo();
            self.input.input.drain(..self.input.cursor_pos);
            self.input.cursor_pos = 0;
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn kill_char(&mut self) {
        if self.input.cursor_pos < self.input.input.len() {
            self.push_undo();
            let end = crate::update::input::next_grapheme_boundary(
                &self.input.input,
                self.input.cursor_pos,
            );
            self.input.input.drain(self.input.cursor_pos..end);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    fn push_undo(&mut self) {
        self.input
            .undo_stack
            .push((self.input.input.clone(), self.input.cursor_pos));
    }

    fn clear_redo(&mut self) {
        self.input.redo_stack.clear();
    }

    pub(crate) fn undo(&mut self) {
        if let Some((text, pos)) = self.input.undo_stack.pop() {
            self.input
                .redo_stack
                .push((self.input.input.clone(), self.input.cursor_pos));
            self.input.input = text;
            self.input.cursor_pos = pos;
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some((text, pos)) = self.input.redo_stack.pop() {
            self.input
                .undo_stack
                .push((self.input.input.clone(), self.input.cursor_pos));
            self.input.input = text;
            self.input.cursor_pos = pos;
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        }
    }

    pub(crate) fn paste(&mut self, text: &str) {
        let clean = text
            .replace("\r\n", "")
            .replace(['\r', '\n'], "")
            .replace('\t', "    ");
        if clean.is_empty() {
            return;
        }
        self.push_undo();
        self.input.input.insert_str(self.input.cursor_pos, &clean);
        self.input.cursor_pos += clean.len();
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn paste_image(&mut self) {
        match crate::clipboard_image::read_clipboard_image() {
            Some(bytes) => {
                let uri = crate::clipboard_image::to_data_uri(&bytes);
                self.session.image_attachments.push(uri);
                self.mark_dirty();
            }
            None => {
                self.input.input_flash = 3;
            }
        }
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn insert_newline(&mut self) {
        self.push_undo();
        if self.input.cursor_pos == self.input.input.len() {
            self.input.input.push('\n');
        } else {
            self.input.input.insert(self.input.cursor_pos, '\n');
        }
        self.input.cursor_pos += 1;
        self.clear_redo();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn push_input(&mut self, c: char) {
        if c == '\t' {
            self.tab_complete();
            return;
        }
        if self.view.vim_nav_mode {
            self.handle_vim_nav_char(c);
            return;
        }
        let is_at_trigger_position = self.input.input.is_empty() || self.input.input.ends_with(' ');
        if is_at_trigger_position && self.completion.path_suggestions.is_none() {
            if self.config.vim_mode {
                if let Some(evt) = self.vim_motion_event(c) {
                    self.update(evt);
                    return;
                }
                if c == '/' {
                    crate::update::dialog::open_command_palette(self);
                    self.mark_dirty();
                    return;
                }
            }
            if c == '@' {
                let needs_brackets = false;
                let cursor = self.input.cursor_pos;
                self.input.file_picker_backup =
                    Some((self.input.input.clone(), cursor, cursor, needs_brackets));
                crate::update::dialog::open_at_file_picker_all(self);
                return;
            }
        }
        self.insert_char(c);
    }

    pub(crate) fn submit(&mut self) {
        if self.completion.at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.completion.path_suggestions.is_some() {
            self.path_completion_select();
            return;
        }
        self.accept_ghost();
        if self.open_dialog.is_some() {
            return;
        }
        let Some(content) = self.take_submit_content() else {
            return;
        };

        self.agent.tokens_in += self.agent.token_tracker.estimate_input(&content);

        if let Some(stripped) = content.strip_prefix('!') {
            let command = stripped.trim().to_string();
            if !command.is_empty() {
                self.run_bash_command(&command);
            }
            return;
        }

        self.add_to_input_history(content.clone());
        self.dispatch_submit_content(content);
    }

    fn take_submit_content(&mut self) -> Option<String> {
        if self.input.input.is_empty() {
            self.input.input_flash = 3;
            return None;
        }
        let content = std::mem::take(&mut self.input.input).trim().to_string();
        self.input.cursor_pos = 0;
        self.input.input_scroll = 0;
        self.input.history_pos = None;
        self.input.undo_stack.clear();
        self.input.redo_stack.clear();
        if crate::update::input::is_quit_command(&content) {
            self.should_quit = true;
            return None;
        }
        if content.is_empty() && self.session.image_attachments.is_empty() {
            return None;
        }
        Some(self.build_content_with_attachments(content))
    }

    fn build_content_with_attachments(&mut self, content: String) -> String {
        if self.session.image_attachments.is_empty() {
            return content;
        }
        let mut full = content;
        for uri in std::mem::take(&mut self.session.image_attachments) {
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
            self.view.scroll = 0;
            self.mark_dirty();
            return;
        }
        if self.agent.turn_active {
            self.agent.message_queue.push(crate::model::QueuedMessage {
                content,
                kind: crate::model::QueuedMessageKind::Steering,
            });
            self.view.scroll = 0;
            self.mark_dirty();
            return;
        }
        let id = self.next_id();
        self.session.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
            ..Default::default()
        });
        self.agent.request_queue.push_back((content, id));
        self.view.scroll = 0;
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
                self.open_dialog = Some(crate::commands::DialogState::PanelStack(*stack));
                self.mark_dirty();
            }
            crate::commands::CommandResult::None => {}
        }
    }

    fn run_bash_command(&mut self, command: &str) {
        let result = crate::update::tools::execute_bash(command);
        let output_msg = format!("$ {}\n{}", command, result);
        self.add_system_msg(output_msg);
        self.view.scroll = 0;
        self.messages_changed();
    }

    pub(crate) fn history_prev(&mut self) {
        if self.input.input_history.is_empty() {
            self.input.input_flash = 3;
            return;
        }
        let pos = match self.input.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => self.input.input_history.len() - 1,
        };
        self.input.history_pos = Some(pos);
        self.input.input = self.input.input_history[pos].clone();
        self.input.cursor_pos = self.input.input.len();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn history_next(&mut self) {
        let pos = match self.input.history_pos {
            Some(p) => p + 1,
            None => {
                self.input.input_flash = 3;
                return;
            }
        };
        if pos >= self.input.input_history.len() {
            self.input.history_pos = None;
            self.input.input.clear();
            self.input.cursor_pos = 0;
        } else {
            self.input.history_pos = Some(pos);
            self.input.input = self.input.input_history[pos].clone();
            self.input.cursor_pos = self.input.input.len();
        }
        self.clamp_input_scroll();
        self.mark_dirty();
    }
}
