impl FeedState {
    fn select_entry(&mut self, direction: i8) {
        let entries = self.selectable_entries();
        if entries.is_empty() {
            self.navigation.selected_entry = None;
            return;
        }
        let current = self
            .navigation
            .selected_entry
            .and_then(|entry| entries.iter().position(|candidate| *candidate == entry));
        let next = match (current, direction) {
            (None, 1) => 0,
            (None, -1) => entries.len() - 1,
            (Some(index), 1) => (index + 1) % entries.len(),
            (Some(0), -1) => entries.len() - 1,
            (Some(index), -1) => index - 1,
            _ => 0,
        };
        self.navigation.selected_entry = Some(entries[next]);
        self.navigation.selected_tool_id = self.lines[entries[next]].tool_call_id.clone();
        self.navigation.selected_tool_row_id = self.lines[entries[next]].tool_row_id;
        self.navigation.detach_from_tail();
    }

}
