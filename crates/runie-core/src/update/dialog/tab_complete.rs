use crate::model::AppState;

impl AppState {
    /// Handle Tab key in the input box — prefix file completion with ghost text.
    /// Shows the rest of the filename in gray; Tab cycles through matches.
    /// - First Tab: show ghost (rest of filename in gray)
    /// - Second Tab with single match: complete (accept ghost)
    /// - Second Tab with multiple matches: cycle to next
    pub(crate) fn tab_complete(&mut self) {
        if self.open_dialog().is_some() {
            return;
        }
        if self.completion_mut().at_suggestions.is_some() {
            self.tab_complete_at_ref();
            return;
        }

        // Phase 1: collect needed data in one mutable borrow
        let cursor;
        let input_len;
        let input_clone;
        {
            let input = self.input_mut();
            cursor = input.cursor_pos;
            input_len = input.input.len();
            input_clone = input.input.clone();
        }

        // Phase 2: pure computation using collected data
        let (prefix, prefix_start) = compute_tab_prefix(cursor, &input_clone);
        let insert_pos = choose_insert_pos(cursor, prefix_start, &prefix, input_len);

        // Phase 3: mutations after all borrows are released
        self.input_mut().file_picker_backup = Some((input_clone, insert_pos, cursor, false));
        crate::update::dialog::open_at_file_picker(self, Some(&prefix));
    }

    /// Clear ghost completion (called on typing, backspace, cursor movement, etc.)
    pub(crate) fn clear_ghost(&mut self) {
        let input = self.input_mut();
        let has_ghost = input.ghost_completion.is_some();
        let has_prefix = input.tab_complete_prefix.is_some();
        let has_matches = !input.tab_complete_matches.is_empty();

        if has_ghost || has_prefix || has_matches {
            input.ghost_completion = None;
            input.tab_complete_prefix = None;
            input.tab_complete_matches.clear();
            input.tab_complete_index = 0;
            self.view_mut().dirty = true;
        }
    }

    /// Accept ghost completion: replace the token being completed with the
    /// full correctly-capitalized match (fixes e.g. "car" → "Cargo.lock").
    pub(crate) fn accept_ghost(&mut self) {
        if self.input().ghost_completion.is_none() {
            return;
        }
        let cursor;
        let input_len;
        let input_clone;
        let tab_complete_index;
        let tab_complete_matches_clone;
        let ghost_clone;
        {
            let input = self.input_mut();
            cursor = input.cursor_pos;
            input_len = input.input.len();
            input_clone = input.input.clone();
            tab_complete_index = input.tab_complete_index;
            tab_complete_matches_clone = input.tab_complete_matches.clone();
            ghost_clone = input.ghost_completion.clone();
        }

        let before_cursor = &input_clone[..cursor.min(input_len)];
        let token_start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);

        let input = self.input_mut();
        if let Some(full) = tab_complete_matches_clone.get(tab_complete_index) {
            input.input = input_clone.clone();
            input.input.replace_range(token_start..cursor, full);
            input.cursor_pos = token_start + full.len();
        } else if let Some(ghost) = ghost_clone {
            input.input = input_clone;
            input.input.push_str(&ghost);
            input.cursor_pos = input.input.len();
        }
        input.ghost_completion = None;
        input.tab_complete_prefix = None;
        input.tab_complete_matches.clear();
        input.tab_complete_index = 0;
        self.view_mut().dirty = true;
    }

    /// Handle Tab key for @ file reference suggestions.
    /// Cycles through the at_suggestions list.
    pub(crate) fn tab_complete_at_ref(&mut self) {
        let suggestions_len;
        let has_suggestions;
        {
            let comp = self.completion_mut();
            has_suggestions = comp.at_suggestions.as_ref().is_some_and(|s| !s.is_empty());
            suggestions_len = comp.at_suggestions.as_ref().map_or(0, |s| s.len());
            if !has_suggestions {
                comp.at_selected = None;
            } else if comp.at_selected.is_none() {
                comp.at_selected = Some(0);
            }
        }

        if !has_suggestions {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
            return;
        }

        let comp = self.completion_mut();
        let current = comp.at_selected.unwrap_or(0);
        comp.at_selected = Some((current + 1) % suggestions_len);
        self.view_mut().dirty = true;
    }
}

/// Pure function to compute the tab prefix at a given cursor position.
fn compute_tab_prefix(cursor: usize, input: &str) -> (String, usize) {
    if input.is_empty() || cursor == 0 {
        return (String::new(), 0);
    }
    let before_cursor = &input[..cursor.min(input.len())];
    let token_start = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
    let prefix = input[token_start..cursor].to_string();
    (prefix, token_start)
}

fn choose_insert_pos(cursor: usize, prefix_start: usize, prefix: &str, len: usize) -> usize {
    let is_at_end = cursor >= len;
    if is_at_end && !prefix.is_empty() {
        prefix_start
    } else {
        cursor
    }
}
