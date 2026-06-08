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

    pub(crate) fn push_input(&mut self, c: char) {
        if c == '\t' {
            if self.input.contains('@') || self.at_suggestions.is_some() {
                self.cycle_at_suggestions();
            }
            return;
        }
        self.input.push(c);
        self.mark_dirty();
        if self.input.contains('@') {
            let query = self.input.split('@').last().unwrap_or("").to_string();
            let should_refresh = self.last_at_query.as_ref() != Some(&query);
            if should_refresh {
                self.last_at_query = Some(query);
                self.refresh_at_suggestions();
            }
        } else {
            self.at_suggestions = None;
            self.at_selected = None;
            self.last_at_query = None;
        }
    }

    pub(crate) fn pop_input(&mut self) {
        self.input.pop();
        self.mark_dirty();
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
        self.at_selected = self.at_selected.map(|i| i.min(suggestions.len() - 1)).or(Some(0));
        self.at_suggestions = Some(suggestions);
        self.mark_dirty();
    }

    fn cycle_at_suggestions(&mut self) {
        let suggestions = match self.at_suggestions.as_mut() {
            Some(s) => s,
            None => {
                self.refresh_at_suggestions();
                return;
            }
        };
        let idx = self.at_selected.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
        self.at_selected = Some(idx);
        self.mark_dirty();
    }

    fn insert_at_suggestion(&mut self) {
        if let Some(idx) = self.at_selected {
            if let Some(suggestions) = self.at_suggestions.take() {
                if let Some(selected) = suggestions.get(idx) {
                    self.input = crate::file_refs::insert_at_ref(&self.input, selected);
                }
            }
            self.at_selected = None;
            self.last_at_query = None;
            self.mark_dirty();
        }
    }
}
