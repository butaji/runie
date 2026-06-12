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

    pub(crate) fn cycle_at_suggestions(&mut self) {
        // Legacy popup disabled — no-op
        self.completion.at_suggestions = None;
    }

    pub(crate) fn insert_at_suggestion(&mut self) {
        // Legacy popup disabled — no-op
        self.completion.at_suggestions = None;
        self.completion.at_selected = None;
    }
}
