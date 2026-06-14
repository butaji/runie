use crate::model::AppState;

impl AppState {
    /// Legacy @-trigger popup disabled — file picker now uses PanelStack dialog.
    /// Clears stale state and ghost completions on any text change.
    pub(crate) fn handle_at_trigger(&mut self) {
        self.clear_ghost();
        self.completion.at_suggestions = None;
        self.completion.at_selected = None;
        self.completion.last_at_query = None;
    }

    /// Insert the currently selected @ suggestion into the input.
    /// Wraps the path in [...] format.
    pub(crate) fn insert_at_suggestion(&mut self) {
        let suggestions = match &self.completion.at_suggestions {
            Some(s) if !s.is_empty() => s,
            _ => {
                // No suggestions, just clear state
                self.completion.at_suggestions = None;
                self.completion.at_selected = None;
                return;
            }
        };

        let selected_idx = self.completion.at_selected.unwrap_or(0);
        if let Some(selected) = suggestions.get(selected_idx) {
            // Insert the selected suggestion wrapped in [...]
            self.input.input.push_str(&format!("[{}]", selected));
            self.input.cursor_pos = self.input.input.len();
        }

        // Clear completion state
        self.completion.at_suggestions = None;
        self.completion.at_selected = None;
        self.mark_dirty();
    }
}
