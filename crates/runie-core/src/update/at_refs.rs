use crate::model::AppState;

impl AppState {
    pub(crate) fn handle_at_trigger(&mut self) {
        if self.input.contains('@') {
            let query = self.input.split('@').next_back().unwrap_or("").to_string();
            let needs_refresh = self.last_at_query.as_ref() != Some(&query)
                || self.at_suggestions.is_none();
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

    pub(crate) fn refresh_at_suggestions(&mut self) {
        let query = self.input.split('@').next_back().unwrap_or("").to_string();
        let mut suggestions = crate::file_refs::complete_at_ref(&self.input, ".", 50);
        if suggestions.len() > 1 && !query.is_empty() {
            let refs: Vec<&str> = suggestions.iter().map(|s| s.as_str()).collect();
            suggestions = crate::fuzzy::fuzzy_filter(&query, &refs, 10)
                .into_iter()
                .map(|s| s.to_string())
                .collect();
        }
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

    pub(crate) fn cycle_at_suggestions(&mut self) {
        if let Some(suggestions) = self.at_suggestions.as_mut() {
            let idx = self.at_selected.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
            self.at_selected = Some(idx);
            self.mark_dirty();
        } else {
            self.refresh_at_suggestions();
        }
    }

    pub(crate) fn insert_at_suggestion(&mut self) {
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
