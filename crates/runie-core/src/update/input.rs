use super::*;

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

    // === Cursor Movement ===

    pub(crate) fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.mark_dirty();
        }
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
            self.mark_dirty();
        }
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.cursor_pos != 0 {
            self.cursor_pos = 0;
            self.mark_dirty();
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.cursor_pos != self.input.len() {
            self.cursor_pos = self.input.len();
            self.mark_dirty();
        }
    }

    // === Text Editing ===

    /// Insert character at cursor position
    pub(crate) fn insert_char(&mut self, c: char) {
        if self.cursor_pos == self.input.len() {
            self.input.push(c);
        } else {
            self.input.insert(self.cursor_pos, c);
        }
        self.cursor_pos += 1;
        self.handle_at_trigger();
        self.mark_dirty();
    }

    /// Delete character before cursor
    pub(crate) fn delete_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    /// Delete word before cursor (Emacs Ctrl+W)
    /// Standard Emacs behavior: delete the word BEFORE cursor
    pub(crate) fn delete_word(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let pos = self.cursor_pos;
        let bytes = self.input.as_bytes();
        
        // Find start of word to delete (going backwards from cursor)
        let mut delete_start = pos;
        
        // Skip trailing spaces before cursor
        while delete_start > 0 && bytes[delete_start - 1] == b' ' {
            delete_start -= 1;
        }
        
        // Skip word characters
        while delete_start > 0 {
            let prev = delete_start - 1;
            if bytes[prev] == b' ' {
                break;
            }
            delete_start -= 1;
        }
        
        // Delete from delete_start to pos
        if delete_start < pos {
            self.input.drain(delete_start..pos);
            self.cursor_pos = delete_start;
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    /// Delete from cursor to end of line (Emacs Ctrl+K)
    pub(crate) fn delete_to_end(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.truncate(self.cursor_pos);
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    /// Delete from start to cursor (Emacs Ctrl+U)
    pub(crate) fn delete_to_start(&mut self) {
        if self.cursor_pos > 0 {
            self.input.drain(..self.cursor_pos);
            self.cursor_pos = 0;
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    /// Delete character at cursor (Emacs Ctrl+D)
    pub(crate) fn kill_char(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
            self.handle_at_trigger();
            self.mark_dirty();
        }
    }

    // === Legacy methods (for backward compatibility) ===

    pub(crate) fn push_input(&mut self, c: char) {
        // Tab cycles @-suggestions if active, otherwise inserts tab
        if c == '\t' {
            if self.input.contains('@') || self.at_suggestions.is_some() {
                self.cycle_at_suggestions();
            }
            return;
        }
        self.insert_char(c);
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn submit(&mut self) {
        if self.at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        self.cursor_pos = 0;
        if content.is_empty() {
            return;
        }
        if let Some(response) = self.handle_slash(&content) {
            self.add_system_msg(response);
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
        });
        self.request_queue.push_back((content, id));
        self.scroll = 0;
        self.messages_changed();
    }

    // === @-ref suggestions ===

    fn handle_at_trigger(&mut self) {
        if self.input.contains('@') {
            let query = self.input.split('@').last().unwrap_or("").to_string();
            // Refresh if query changed OR if suggestions not yet populated
            let needs_refresh = self.last_at_query.as_ref() != Some(&query) 
                || self.at_suggestions.is_none();
            eprintln!("DEBUG handle_at_trigger: input={:?}, query={:?}, needs_refresh={}", self.input, query, needs_refresh);
            if needs_refresh {
                self.last_at_query = Some(query.clone());
                self.refresh_at_suggestions();
            }
        } else {
            self.at_suggestions = None;
            self.at_selected = None;
            self.last_at_query = None;
        }
    }

    fn refresh_at_suggestions(&mut self) {
        let mut suggestions = crate::file_refs::complete_at_ref(&self.input, ".", 10);
        if suggestions.is_empty() {
            suggestions = crate::file_refs::find_files("", ".", 10);
        }
        if suggestions.is_empty() {
            self.at_suggestions = None;
            self.at_selected = None;
            return;
        }
        self.at_suggestions = Some(suggestions);
        self.at_selected = Some(0);
        self.mark_dirty();
    }

    fn cycle_at_suggestions(&mut self) {
        if let Some(suggestions) = self.at_suggestions.as_mut() {
            let idx = self.at_selected.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
            self.at_selected = Some(idx);
            self.mark_dirty();
        } else {
            self.refresh_at_suggestions();
        }
    }

    fn insert_at_suggestion(&mut self) {
        if let Some(idx) = self.at_selected {
            if let Some(suggestions) = self.at_suggestions.take() {
                if let Some(selected) = suggestions.get(idx) {
                    self.input = crate::file_refs::insert_at_ref(&self.input, selected);
                    self.cursor_pos = self.input.len();
                }
            }
            self.at_selected = None;
            self.last_at_query = None;
            self.mark_dirty();
        }
    }
}
