use super::input_text::{
    find_word_boundary_left, find_word_boundary_right, next_grapheme_boundary,
    prev_grapheme_boundary,
};
use super::*;
use crate::model::{ChatMessage, Role};

impl AppState {
    pub fn hint_text(&self) -> String {
        let mut parts = Vec::new();
        parts.push("ctrl+o expand/collapse".to_string());

        if self.vim_nav_mode {
            parts.push("j down · k up".to_string());
            parts.push("g/G top/bottom".to_string());
            parts.push("space/i input".to_string());
            parts.push("esc input".to_string());
        } else if self.completion.at_suggestions.is_some() {
            parts.push("tab cycle".to_string());
            parts.push("enter insert".to_string());
            parts.push("esc close".to_string());
        } else if self.agent.turn_active {
            parts.push("enter steer".to_string());
            parts.push("alt+enter follow-up".to_string());
            parts.push(if self.config.vim_mode {
                "esc abort·nav".to_string()
            } else {
                "esc abort".to_string()
            });
        } else if !self.input.input.is_empty() {
            parts.push("enter send".to_string());
            parts.push("alt+enter follow-up".to_string());
            parts.push("esc clear".to_string());
        } else if self.config.vim_mode {
            parts.push("esc nav".to_string());
        } else {
            parts.push("alt+enter follow-up".to_string());
            parts.push("esc clear".to_string());
        }
        parts.push("ctrl+c quit".to_string());
        parts.join(" · ")
    }

    // === Cursor Movement (grapheme-aware) ===

    pub(crate) fn cursor_left(&mut self) {
        if self.input.cursor_pos > 0 {
            self.input.cursor_pos =
                prev_grapheme_boundary(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_right(&mut self) {
        // If ghost completion is active, accept it instead of moving cursor
        if self.input.ghost_completion.is_some() {
            self.accept_ghost();
            return;
        }
        if self.input.cursor_pos < self.input.input.len() {
            self.input.cursor_pos =
                next_grapheme_boundary(&self.input.input, self.input.cursor_pos);
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input.input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.input.cursor_pos != 0 {
            self.input.cursor_pos = 0;
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.input.input.contains('\n') {
            self.move_cursor_to_line_end();
        } else if self.input.cursor_pos != self.input.input.len() {
            self.input.cursor_pos = self.input.input.len();
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    // === Word Navigation ===

    pub(crate) fn cursor_word_left(&mut self) {
        if self.input.cursor_pos > 0 {
            self.input.cursor_pos =
                find_word_boundary_left(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_right(&mut self) {
        if self.input.cursor_pos < self.input.input.len() {
            self.input.cursor_pos =
                find_word_boundary_right(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    // === Text Editing ===

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
            // Check if the character before cursor is a newline
            let char_before_cursor = self.input.input[..self.input.cursor_pos].chars().last();
            if char_before_cursor == Some('\n') {
                // Remove the newline character (join lines)
                self.push_undo();
                let new_pos = self.input.cursor_pos - 1; // Position after removing newline
                self.input.input.remove(self.input.cursor_pos - 1);
                self.input.cursor_pos = new_pos;
                self.clear_redo();
                self.handle_at_trigger();
                self.clamp_input_scroll();
                self.mark_dirty();
            } else {
                // Normal delete - remove grapheme before cursor
                self.push_undo();
                let new_pos = prev_grapheme_boundary(&self.input.input, self.input.cursor_pos);
                self.input.input.drain(new_pos..self.input.cursor_pos);
                self.input.cursor_pos = new_pos;
                self.clear_redo();
                self.handle_at_trigger();
                self.clamp_input_scroll();
                self.mark_dirty();
            }
        } else if self.input.cursor_pos == 0 && !self.input.input.is_empty() {
            // Cursor at absolute start - check if there's a newline
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
        let start = find_word_boundary_left(&self.input.input, self.input.cursor_pos);
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
            let end = next_grapheme_boundary(&self.input.input, self.input.cursor_pos);
            self.input.input.drain(self.input.cursor_pos..end);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    // === Undo / Redo ===

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

    // === Paste ===

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
                // Clipboard does not contain an image — flash to indicate.
                self.input.input_flash = 3;
            }
        }
    }

    // === Legacy methods ===

    pub(crate) fn push_input(&mut self, c: char) {
        if c == '\t' {
            self.tab_complete();
            return;
        }
        // Vim nav mode intercepts printable characters as motions.
        if self.vim_nav_mode {
            self.handle_vim_nav_char(c);
            return;
        }
        // When input is empty OR ends with space, single-key shortcuts bypass typing.
        let is_at_trigger_position = self.input.input.is_empty()
            || self.input.input.ends_with(' ');
        if is_at_trigger_position && self.completion.path_suggestions.is_none() {
            if self.config.vim_mode {
                if let Some(evt) = self.vim_motion_event(c) {
                    self.update(evt);
                    return;
                }
            }
            match c {
                '/' => {
                    super::dialog::open_command_palette(self);
                    self.mark_dirty();
                    return;
                }
                '@' => {
                    // Save @ as part of the filter/prefix
                    self.file_picker_backup = Some((self.input.input.clone(), self.input.cursor_pos));
                    super::dialog::open_at_file_picker_all(self);
                    return;
                }
                _ => {}
            }
        }
        self.insert_char(c);
    }

    /// Handle a character while in vim nav mode. Returns true if the char
    /// was consumed (motion or space-to-leave), false if it should be
    /// treated as a normal typed character (which also leaves nav mode).
    fn handle_vim_nav_char(&mut self, c: char) {
        if c == ' ' {
            self.vim_nav_mode = false;
            self.insert_char(' ');
            return;
        }
        if c == 'i' {
            // Vim "insert" key: exit nav mode WITHOUT inserting 'i'.
            self.vim_nav_mode = false;
            self.mark_dirty();
            return;
        }
        // Element-level navigation: `j` jumps DOWN (newer, toward
        // bottom), `k` jumps UP (older, toward top). g/G go to
        // top/bottom. The non-nav-mode `vim_motion_event` still does
        // line-level scroll, so we handle nav mode separately here.
        match c {
            'j' => {
                let last = self.view.posts.len().saturating_sub(1);
                if self.view.selected_post.unwrap_or(0) >= last {
                    // Already at the lowest post. The "next thing"
                    // is the input box itself — re-enable it.
                    self.vim_nav_mode = false;
                    self.mark_dirty();
                    return;
                }
                super::scroll::element_jump_down(self);
                return;
            }
            'k' => {
                if self.view.selected_post.unwrap_or(0) == 0 {
                    self.input.input_flash = 3;
                    self.mark_dirty();
                    return;
                }
                super::scroll::element_jump_up(self);
                return;
            }
            'g' => {
                self.update(Event::GoToTop);
                return;
            }
            'G' => {
                self.update(Event::GoToBottom);
                return;
            }
            _ => {}
        }
        if let Some(evt) = self.vim_motion_event(c) {
            self.update(evt);
            return;
        }
        // Any other printable character exits nav mode and is typed.
        self.vim_nav_mode = false;
        self.insert_char(c);
    }

    /// Called by `update` for non-char events while in nav mode. Returns
    /// `Some(false)` to fully consume the event, `Some(true)` to fall
    /// through.
    pub(crate) fn handle_vim_nav_event(&mut self, event: &Event) -> Option<bool> {
        match event {
            // ArrowUp / k => up (older); ArrowDown / j => down (newer).
            Event::HistoryPrev | Event::ScrollUp => {
                if self.view.selected_post.unwrap_or(0) == 0 {
                    self.input.input_flash = 3;
                    self.mark_dirty();
                } else {
                    super::scroll::element_jump_up(self);
                }
                Some(false)
            }
            Event::HistoryNext | Event::ScrollDown => {
                let last = self.view.posts.len().saturating_sub(1);
                if self.view.selected_post.unwrap_or(0) >= last {
                    // At the lowest post: the next focus is the
                    // input box. Exit nav mode instead of flashing.
                    self.vim_nav_mode = false;
                    self.mark_dirty();
                } else {
                    super::scroll::element_jump_down(self);
                }
                Some(false)
            }
            Event::PageUp | Event::PageDown | Event::GoToTop | Event::GoToBottom => {
                super::scroll::scroll_event(self, event.clone());
                Some(false)
            }
            Event::ToggleCommandPalette => {
                super::dialog_toggle::dialog_toggle_event(self, event.clone());
                Some(false)
            }
            Event::CopyLastResponse => {
                super::control::control_event(self, event.clone());
                Some(false)
            }
            _ => Some(true),
        }
    }

    /// Map a single character to a vim motion event when vim_mode is on and
    /// the input field is empty. Returns None for characters that should be
    /// inserted normally.
    fn vim_motion_event(&self, c: char) -> Option<Event> {
        match c {
            'j' => Some(Event::ScrollUp),
            'k' => Some(Event::ScrollDown),
            'g' => Some(Event::GoToTop),
            'G' => Some(Event::GoToBottom),
            '/' => Some(Event::ToggleCommandPalette),
            // 'y' is NOT a motion — it's typed normally. Copy-last is
            // bound to Ctrl+Shift+O.
            _ => None,
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

    pub(crate) fn submit(&mut self) {
        if self.completion.at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.completion.path_suggestions.is_some() {
            self.path_completion_select();
            return;
        }
        // Accept ghost completion before submitting
        self.accept_ghost();
        if self.input.input.is_empty() {
            self.input.input_flash = 3;
            return;
        }
        let content = std::mem::take(&mut self.input.input).trim().to_string();
        self.input.cursor_pos = 0;
        self.input.input_scroll = 0;
        self.input.history_pos = None;
        self.input.undo_stack.clear();
        self.input.redo_stack.clear();
        if content.is_empty() && self.session.image_attachments.is_empty() {
            return;
        }

        // Quit commands: typing these in the input box exits the app.
        if content.eq_ignore_ascii_case("quit")
            || content.eq_ignore_ascii_case("exit")
            || content.eq_ignore_ascii_case(":q")
        {
            self.should_quit = true;
            return;
        }

        // Append image attachments to content
        let content = if self.session.image_attachments.is_empty() {
            content
        } else {
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
        };

        // Count input tokens
        self.agent.tokens_in += crate::model::count_tokens(&content);

        // Handle bash prefix (!)
        if let Some(stripped) = content.strip_prefix('!') {
            let command = stripped.trim().to_string();
            if !command.is_empty() {
                self.run_bash_command(&command);
            }
            return;
        }

        // Add to history and persist
        self.add_to_input_history(content.clone());
        if let Some(result) = self.handle_slash(&content) {
            match result {
                crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
                crate::commands::CommandResult::Warning(msg) => {
                    self.notify(msg, crate::event::TransientLevel::Warning)
                }
                crate::commands::CommandResult::Event(evt) => self.update(evt),
                crate::commands::CommandResult::OpenDialog(d) => match d {
                    crate::commands::DialogType::CommandPalette => {
                        super::dialog::open_command_palette(self)
                    }
                    crate::commands::DialogType::ModelSelector => {
                        super::dialog::open_model_selector(self)
                    }
                    crate::commands::DialogType::Settings => {
                        super::dialog::open_settings_dialog(self)
                    }
                    crate::commands::DialogType::ScopedModels => {
                        super::dialog::open_scoped_models_dialog(self)
                    }
                },
                crate::commands::CommandResult::OpenPanelStack(stack) => {
                    self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
                    self.mark_dirty();
                }
                crate::commands::CommandResult::None => {}
            }
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

    /// Run a bash command and display output
    fn run_bash_command(&mut self, command: &str) {
        let result = bash::execute_bash(command);
        let output_msg = format!("$ {}\n{}", command, result);
        self.add_system_msg(output_msg);
        self.view.scroll = 0;
        self.messages_changed();
    }

    // === Input History ===

    pub(crate) fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            self.input.input_flash = 3;
            return;
        }
        let pos = match self.input.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => self.input_history.len() - 1,
        };
        self.input.history_pos = Some(pos);
        self.input.input = self.input_history[pos].clone();
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
        if pos >= self.input_history.len() {
            self.input.history_pos = None;
            self.input.input.clear();
            self.input.cursor_pos = 0;
        } else {
            self.input.history_pos = Some(pos);
            self.input.input = self.input_history[pos].clone();
            self.input.cursor_pos = self.input.input.len();
        }
        self.clamp_input_scroll();
        self.mark_dirty();
    }
}
