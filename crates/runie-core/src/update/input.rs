use unicode_segmentation::UnicodeSegmentation;
use super::*;
// === Grapheme helpers ===

fn prev_grapheme_boundary(s: &str, pos: usize) -> usize {
    let mut last = 0;
    for (i, _) in s.grapheme_indices(true) {
        if i >= pos { break; }
        last = i;
    }
    last
}

fn next_grapheme_boundary(s: &str, pos: usize) -> usize {
    for (i, _) in s.grapheme_indices(true) {
        if i > pos { return i; }
    }
    s.len()
}

// === Word boundary helpers ===

fn find_word_boundary_left(s: &str, pos: usize) -> usize {
    let mut pos = pos;
    while pos > 0 {
        let prev = prev_grapheme_boundary(s, pos);
        if &s[prev..pos] != " " { break; }
        pos = prev;
    }
    while pos > 0 {
        let prev = prev_grapheme_boundary(s, pos);
        if &s[prev..pos] == " " { break; }
        pos = prev;
    }
    pos
}

fn find_word_boundary_right(s: &str, pos: usize) -> usize {
    let mut pos = pos;
    let len = s.len();
    while pos < len {
        let next = next_grapheme_boundary(s, pos);
        if &s[pos..next] == " " { break; }
        pos = next;
    }
    while pos < len {
        let next = next_grapheme_boundary(s, pos);
        if &s[pos..next] != " " { break; }
        pos = next;
    }
    pos
}

impl AppState {
    pub fn hint_text(&self) -> String {
        let mut parts = Vec::new();
        parts.push("Ctrl+Shift+E=expand/collapse".to_string());
        if self.at_suggestions.is_some() {
            parts.push("Tab=cycle".to_string());
            parts.push("Enter=insert".to_string());
            parts.push("Esc=close".to_string());
        } else if self.turn_active {
            parts.push("Enter=steer".to_string());
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=abort".to_string());
        } else if !self.input.is_empty() {
            parts.push("Enter=send".to_string());
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=clear".to_string());
        } else {
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=clear".to_string());
        }
        parts.push("Ctrl+C=quit".to_string());
        parts.join(" | ")
    }

    // === Cursor Movement (grapheme-aware) ===

    pub(crate) fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = prev_grapheme_boundary(&self.input, self.cursor_pos);
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = next_grapheme_boundary(&self.input, self.cursor_pos);
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.cursor_pos != 0 {
            self.cursor_pos = 0;
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.input.contains('\n') {
            self.move_cursor_to_line_end();
        } else if self.cursor_pos != self.input.len() {
            self.cursor_pos = self.input.len();
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    // === Word Navigation ===

    pub(crate) fn cursor_word_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = find_word_boundary_left(&self.input, self.cursor_pos);
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = find_word_boundary_right(&self.input, self.cursor_pos);
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    // === Text Editing ===

    pub(crate) fn insert_char(&mut self, c: char) {
        self.push_undo();
        if self.cursor_pos == self.input.len() {
            self.input.push(c);
        } else {
            self.input.insert(self.cursor_pos, c);
        }
        self.cursor_pos += c.len_utf8();
        self.clear_redo();
        self.handle_at_trigger();
        self.mark_dirty();
    }

    pub(crate) fn delete_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            // Check if the character before cursor is a newline
            let char_before_cursor = self.input[..self.cursor_pos].chars().last();
            if char_before_cursor == Some('\n') {
                // Remove the newline character (join lines)
                self.push_undo();
                let new_pos = self.cursor_pos - 1; // Position after removing newline
                self.input.remove(self.cursor_pos - 1);
                self.cursor_pos = new_pos;
                self.clear_redo();
                self.handle_at_trigger();
                self.mark_dirty();
            } else {
                // Normal delete - remove grapheme before cursor
                self.push_undo();
                let new_pos = prev_grapheme_boundary(&self.input, self.cursor_pos);
                self.input.drain(new_pos..self.cursor_pos);
                self.cursor_pos = new_pos;
                self.clear_redo();
                self.handle_at_trigger();
                self.mark_dirty();
            }
        } else if self.cursor_pos == 0 && !self.input.is_empty() {
            // Cursor at absolute start - check if there's a newline
            if self.input.starts_with('\n') {
                self.push_undo();
                self.input.remove(0);
                self.clear_redo();
                self.handle_at_trigger();
                self.mark_dirty();
            } else {
                self.input_flash = 3;
            }
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn delete_word(&mut self) {
        if self.cursor_pos == 0 {
            self.input_flash = 3;
            return;
        }
        self.push_undo();
        let start = find_word_boundary_left(&self.input, self.cursor_pos);
        self.input.drain(start..self.cursor_pos);
        self.cursor_pos = start;
        self.clear_redo();
        self.handle_at_trigger();
        self.mark_dirty();
    }

    pub(crate) fn delete_to_end(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.push_undo();
            self.input.truncate(self.cursor_pos);
            self.clear_redo();
            self.handle_at_trigger();
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn delete_to_start(&mut self) {
        if self.cursor_pos > 0 {
            self.push_undo();
            self.input.drain(..self.cursor_pos);
            self.cursor_pos = 0;
            self.clear_redo();
            self.handle_at_trigger();
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    pub(crate) fn kill_char(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.push_undo();
            let end = next_grapheme_boundary(&self.input, self.cursor_pos);
            self.input.drain(self.cursor_pos..end);
            self.clear_redo();
            self.handle_at_trigger();
            self.mark_dirty();
        } else {
            self.input_flash = 3;
        }
    }

    // === Undo / Redo ===

    fn push_undo(&mut self) {
        self.undo_stack.push((self.input.clone(), self.cursor_pos));
    }

    fn clear_redo(&mut self) {
        self.redo_stack.clear();
    }

    pub(crate) fn undo(&mut self) {
        if let Some((text, pos)) = self.undo_stack.pop() {
            self.redo_stack.push((self.input.clone(), self.cursor_pos));
            self.input = text;
            self.cursor_pos = pos;
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some((text, pos)) = self.redo_stack.pop() {
            self.undo_stack.push((self.input.clone(), self.cursor_pos));
            self.input = text;
            self.cursor_pos = pos;
            self.handle_at_trigger();
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
        self.input.insert_str(self.cursor_pos, &clean);
        self.cursor_pos += clean.len();
        self.clear_redo();
        self.handle_at_trigger();
        self.mark_dirty();
    }

    // === Legacy methods ===

    pub(crate) fn push_input(&mut self, c: char) {
        if c == '\t' {
            if self.input.contains('@') || self.at_suggestions.is_some() {
                self.cycle_at_suggestions();
                return;
            }
            if self.path_suggestions.is_some() {
                self.path_completion_down();
                return;
            }
            self.toggle_path_completion();
            return;
        }
        self.insert_char(c);
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn insert_newline(&mut self) {
        self.push_undo();
        if self.cursor_pos == self.input.len() {
            self.input.push('\n');
        } else {
            self.input.insert(self.cursor_pos, '\n');
        }
        self.cursor_pos += 1;
        self.clear_redo();
        self.mark_dirty();
    }

    pub(crate) fn submit(&mut self) {
        if self.at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.path_suggestions.is_some() {
            self.path_completion_select();
            return;
        }
        if self.input.is_empty() {
            self.input_flash = 3;
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        self.cursor_pos = 0;
        self.history_pos = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        if content.is_empty() {
            return;
        }

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
                crate::commands::CommandResult::Event(evt) => self.update(evt),
                crate::commands::CommandResult::OpenDialog(d) => {
                    self.open_dialog = Some(match d {
                        crate::commands::Dialog::CommandPalette => crate::commands::DialogState::CommandPalette { filter: String::new(), selected: 0 },
                        crate::commands::Dialog::ModelSelector => crate::commands::DialogState::ModelSelector { filter: String::new(), selected: 0 },
                        crate::commands::Dialog::Settings => crate::commands::DialogState::Settings {
                            category: crate::settings::SettingsCategory::Models,
                            selected: 0,
                        },
                        crate::commands::Dialog::ScopedModels => crate::commands::DialogState::ScopedModels { selected: 0 },
                    });
                    self.mark_dirty();
                }
                crate::commands::CommandResult::None => {}
            }
            return;
        }
        if self.turn_active {
            self.message_queue.push(crate::model::QueuedMessage {
                content,
                kind: crate::model::QueuedMessageKind::Steering,
            });
            self.scroll = 0;
            self.mark_dirty();
            return;
        }
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
            ..Default::default()
        });
        self.request_queue.push_back((content, id));
        self.scroll = 0;
        self.messages_changed();
    }

    /// Run a bash command and display output
    fn run_bash_command(&mut self, command: &str) {
        let result = bash::execute_bash(command);
        let output_msg = format!("$ {}\n{}", command, result);
        self.add_system_msg(output_msg);
        self.scroll = 0;
        self.messages_changed();
    }

    // === Input History ===

    pub(crate) fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            self.input_flash = 3;
            return;
        }
        let pos = match self.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => self.input_history.len() - 1,
        };
        self.history_pos = Some(pos);
        self.input = self.input_history[pos].clone();
        self.cursor_pos = self.input.len();
        self.mark_dirty();
    }

    pub(crate) fn history_next(&mut self) {
        let pos = match self.history_pos {
            Some(p) => p + 1,
            None => {
                self.input_flash = 3;
                return;
            }
        };
        if pos >= self.input_history.len() {
            self.history_pos = None;
            self.input.clear();
            self.cursor_pos = 0;
        } else {
            self.history_pos = Some(pos);
            self.input = self.input_history[pos].clone();
            self.cursor_pos = self.input.len();
        }
        self.mark_dirty();
    }

}
