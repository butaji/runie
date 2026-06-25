use crate::model::AppState;

impl AppState {
    /// Handle Tab key in the input box — prefix file completion with ghost text.
    /// Shows the rest of the filename in gray; Tab cycles through matches.
    /// - First Tab: show ghost (rest of filename in gray)
    /// - Second Tab with single match: complete (accept ghost)
    /// - Second Tab with multiple matches: cycle to next
    pub(crate) fn tab_complete(&mut self) {
        if self.open_dialog.is_some() {
            return;
        }
        if self.completion.at_suggestions.is_some() {
            self.tab_complete_at_ref();
            return;
        }

        let cursor = self.input.cursor_pos;
        let (prefix, prefix_start) = self.compute_tab_prefix(cursor);
        let insert_pos = choose_insert_pos(cursor, prefix_start, &prefix, self.input.input.len());
        // File picker always inserts plain paths, never wrapped in brackets
        let needs_brackets = false;

        self.input.file_picker_backup =
            Some((self.input.input.clone(), insert_pos, cursor, needs_brackets));
        crate::update::dialog::open_at_file_picker(self, Some(&prefix));
    }

    fn compute_tab_prefix(&self, cursor: usize) -> (String, usize) {
        if self.input.input.is_empty() || cursor == 0 {
            return (String::new(), 0);
        }
        let before_cursor = &self.input.input[..cursor.min(self.input.input.len())];
        let token_start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = self.input.input[token_start..cursor].to_string();
        (prefix, token_start)
    }

    /// Clear ghost completion (called on typing, backspace, cursor movement, etc.)
    pub(crate) fn clear_ghost(&mut self) {
        // Check if any ghost/completion state exists
        let has_ghost = self.input.ghost_completion.is_some();
        let has_prefix = self.input.tab_complete_prefix.is_some();
        let has_matches = !self.input.tab_complete_matches.is_empty();

        if has_ghost || has_prefix || has_matches {
            self.input.ghost_completion = None;
            self.input.tab_complete_prefix = None;
            self.input.tab_complete_matches.clear();
            self.input.tab_complete_index = 0;
            self.view.dirty = true;
        }
    }

    /// Accept ghost completion: replace the token being completed with the
    /// full correctly-capitalized match (fixes e.g. "car" → "Cargo.lock").
    pub(crate) fn accept_ghost(&mut self) {
        if self.input.ghost_completion.is_none() {
            return;
        }
        let cursor = self.input.cursor_pos;
        let before_cursor = &self.input.input[..cursor.min(self.input.input.len())];
        let token_start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let match_idx = self.input.tab_complete_index;

        if let Some(full) = self.input.tab_complete_matches.get(match_idx) {
            self.input.input.replace_range(token_start..cursor, full);
            self.input.cursor_pos = token_start + full.len();
        } else if let Some(ghost) = self.input.ghost_completion.take() {
            // Fallback when matches aren't set (tests, direct usage)
            self.input.input.push_str(&ghost);
            self.input.cursor_pos = self.input.input.len();
        }
        self.input.ghost_completion = None;
        self.input.tab_complete_prefix = None;
        self.input.tab_complete_matches.clear();
        self.input.tab_complete_index = 0;
        self.view.dirty = true;
    }

    /// Handle Tab key for @ file reference suggestions.
    /// Cycles through the at_suggestions list.
    pub(crate) fn tab_complete_at_ref(&mut self) {
        let suggestions = match &self.completion.at_suggestions {
            Some(s) if !s.is_empty() => s,
            _ => {
                // No suggestions available, just flash
                self.input.input_flash = 3;
                self.view.dirty = true;
                return;
            }
        };

        // Initialize selection to first item if not set
        if self.completion.at_selected.is_none() {
            self.completion.at_selected = Some(0);
        }

        // Cycle to next suggestion
        let current = self.completion.at_selected.unwrap_or(0);
        let next = (current + 1) % suggestions.len();
        self.completion.at_selected = Some(next);
        self.view.dirty = true;
    }

    #[allow(dead_code)]
    fn find_prefix_file_matches(&self, prefix: &str) -> Vec<String> {
        let base = std::env::current_dir().unwrap_or_default();
        let Ok(entries) = std::fs::read_dir(&base) else {
            return Vec::new();
        };

        let prefix_lower = prefix.to_lowercase();
        let mut matches: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if name.to_lowercase().starts_with(&prefix_lower) {
                let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
                let display = if is_dir { format!("{}/", name) } else { name };
                matches.push(display);
            }
        }
        matches.sort();
        matches
    }
}

fn choose_insert_pos(cursor: usize, prefix_start: usize, prefix: &str, len: usize) -> usize {
    let is_at_end = cursor >= len;
    if is_at_end && !prefix.is_empty() {
        prefix_start
    } else {
        cursor
    }
}

#[allow(dead_code)]
fn suffix_after_prefix(prefix: &str, full: &str) -> String {
    if prefix.is_empty() {
        return full.to_string();
    }
    let prefix_lower = prefix.to_lowercase();
    let full_lower = full.to_lowercase();
    if let Some(pos) = full_lower.find(&prefix_lower) {
        let end = pos + prefix.len();
        return full[end..].to_string();
    }
    full.to_string()
}
