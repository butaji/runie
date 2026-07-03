use crate::model::AppState;

impl AppState {
    /// Legacy @-trigger popup disabled — file picker now uses PanelStack dialog.
    /// Clears stale state and ghost completions on any text change.
    pub(crate) fn handle_at_trigger(&mut self) {
        self.clear_ghost();
        self.completion_mut().at_suggestions = None;
        self.completion_mut().at_selected = None;
        self.completion_mut().last_at_query = None;
    }

    /// Insert the currently selected @ suggestion into the input.
    /// Wraps the path in [...] format.
    pub(crate) fn insert_at_suggestion(&mut self) {
        let suggestions = match self.completion_mut().at_suggestions.clone() {
            Some(s) if !s.is_empty() => s,
            _ => {
                // No suggestions, just clear state
                self.completion_mut().at_suggestions = None;
                self.completion_mut().at_selected = None;
                return;
            }
        };

        let selected_idx = self.completion_mut().at_selected.unwrap_or(0);
        if let Some(selected) = suggestions.get(selected_idx) {
            // Insert the selected suggestion wrapped in [...]
            self.input_mut().input.push_str(&format!("[{}]", selected));
            self.input_mut().cursor_pos = self.input_mut().input.len();
        }

        // Clear completion state
        self.completion_mut().at_suggestions = None;
        self.completion_mut().at_selected = None;
        self.view_mut().dirty = true;
    }
}
