use crate::model::AppState;

impl AppState {
    /// Handle Tab key in the input box — prefix file completion with ghost text.
    /// Shows the rest of the filename in gray; Tab cycles through matches.
    /// - First Tab: show ghost (rest of filename in gray)
    /// - Second Tab with single match: complete (accept ghost)
    /// - Second Tab with multiple matches: cycle to next
    pub(crate) fn tab_complete(&mut self) {
        let input = &self.input.input;
        let cursor = self.input.cursor_pos;

        if input.is_empty() || cursor == 0 {
            self.input.input_flash = 3;
            return;
        }

        let before_cursor = &input[..cursor.min(input.len())];
        let token_start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = &input[token_start..cursor];

        if prefix.is_empty() {
            self.input.input_flash = 3;
            return;
        }

        // Cycle if same prefix
        if let Some(ref stored) = self.input.tab_complete_prefix {
            if stored == prefix && !self.input.tab_complete_matches.is_empty() {
                let match_count = self.input.tab_complete_matches.len();

                // If only 1 match, accept the ghost (complete the word)
                if match_count == 1 {
                    self.accept_ghost();
                    return;
                }

                // Multiple matches: cycle to next
                let next_idx = (self.input.tab_complete_index + 1) % match_count;
                let full = self.input.tab_complete_matches[next_idx].clone();
                self.input.ghost_completion = Some(suffix_after_prefix(prefix, &full));
                self.input.tab_complete_index = next_idx;
                self.mark_dirty();
                return;
            }
        }

        // Find new matches
        let matches = self.find_prefix_file_matches(prefix);
        if matches.is_empty() {
            self.input.input_flash = 3;
            return;
        }

        // If only 1 match, complete immediately
        if matches.len() == 1 {
            let full = matches[0].clone();
            self.input.ghost_completion = Some(suffix_after_prefix(prefix, &full));
            self.input.tab_complete_prefix = Some(prefix.to_string());
            self.input.tab_complete_matches = matches;
            self.input.tab_complete_index = 0;
            self.mark_dirty();
            return;
        }

        // Multiple matches: show ghost and wait for cycling
        let full = matches[0].clone();
        self.input.ghost_completion = Some(suffix_after_prefix(prefix, &full));
        self.input.tab_complete_prefix = Some(prefix.to_string());
        self.input.tab_complete_matches = matches;
        self.input.tab_complete_index = 0;
        self.mark_dirty();
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
            self.mark_dirty();
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
        self.mark_dirty();
    }

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
