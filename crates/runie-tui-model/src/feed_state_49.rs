impl FeedState {
    fn select_tool(&mut self, direction: i8) {
        let entries = self.tool_selection_entries();
        if entries.is_empty() {
            self.navigation.selected_tool_id = None;
            self.navigation.selected_tool_row_id = None;
            return;
        }
        let current = self.navigation.selected_tool_id.as_ref().and_then(|id| {
            self.navigation.selected_entry.and_then(|entry| {
                entries.iter().position(|candidate| {
                    *candidate == entry
                        && self.lines[*candidate].tool_call_id.as_deref() == Some(id)
                })
            })
        });
        let next = match (current, direction) {
            (None, 1) => 0,
            (None, -1) => entries.len() - 1,
            (Some(index), 1) => (index + 1) % entries.len(),
            (Some(0), -1) => entries.len() - 1,
            (Some(index), -1) => index - 1,
            _ => 0,
        };
        let selected_entry = entries[next];
        let selected_id = self.lines[selected_entry].tool_call_id.clone();
        self.navigation.selected_tool_id = selected_id.clone();
        self.navigation.selected_entry = Some(selected_entry);
        self.navigation.selected_tool_row_id = self.lines[selected_entry].tool_row_id;
        let selected_id = selected_id.unwrap_or_default();
        self.reveal_dense_group(&selected_id);
    }

}
