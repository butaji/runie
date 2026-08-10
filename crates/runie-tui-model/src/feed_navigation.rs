// Transcript, tool, and dense-group navigation state transitions.

impl FeedState {
    fn select_entry(&mut self, direction: i8) {
        let entries = self.selectable_entries();
        if entries.is_empty() { self.navigation.selected_entry = None; return; }
        let current = self.navigation.selected_entry.and_then(|entry| entries.iter().position(|candidate| *candidate == entry));
        let next = match (current, direction) {
            (None, 1) => 0, (None, -1) => entries.len() - 1,
            (Some(index), 1) => (index + 1) % entries.len(),
            (Some(0), -1) => entries.len() - 1, (Some(index), -1) => index - 1, _ => 0,
        };
        self.navigation.selected_entry = Some(entries[next]);
        self.navigation.selected_tool_id = self.lines[entries[next]].tool_call_id.clone();
        self.navigation.selected_tool_row_id = self.lines[entries[next]].tool_row_id;
        self.navigation.detach_from_tail();
    }

    fn tool_selection_entries(&self) -> Vec<usize> {
        let mut seen = HashSet::new();
        self.lines.iter().enumerate().filter_map(|(index, line)| {
            (line.kind.is_tool_header() && line.tool_call_id.is_some() && seen.insert(tool_member_key(&self.lines, index))).then_some(index)
        }).collect()
    }

    fn select_tool(&mut self, direction: i8) {
        let entries = self.tool_selection_entries();
        if entries.is_empty() { self.navigation.selected_tool_id = None; self.navigation.selected_tool_row_id = None; return; }
        let current = self.navigation.selected_tool_id.as_ref().and_then(|id| self.navigation.selected_entry.and_then(|entry| entries.iter().position(|candidate| *candidate == entry && self.lines[*candidate].tool_call_id.as_deref() == Some(id))));
        let next = match (current, direction) {
            (None, 1) => 0, (None, -1) => entries.len() - 1,
            (Some(index), 1) => (index + 1) % entries.len(),
            (Some(0), -1) => entries.len() - 1, (Some(index), -1) => index - 1, _ => 0,
        };
        let selected_entry = entries[next];
        let selected_id = self.lines[selected_entry].tool_call_id.clone();
        self.navigation.selected_tool_id = selected_id.clone();
        self.navigation.selected_entry = Some(selected_entry);
        self.navigation.selected_tool_row_id = self.lines[selected_entry].tool_row_id;
        self.reveal_dense_group(&selected_id.unwrap_or_default());
    }

    fn reveal_dense_group(&mut self, tool_id: &str) {
        let Some(member_index) = self.lines.iter().position(|line| line.tool_call_id.as_deref() == Some(tool_id)) else { return; };
        let ids = self.dense_group_ids(member_index);
        if ids.len() > GROK_GROUP_MAX_VISIBLE {
            self.navigation.revealed_dense_groups.insert(ids[0].clone());
            self.navigation.selected_entry = Some(member_index);
            self.navigation.center_revealed_entry = true;
        }
    }

    fn dense_group_ids(&self, member_index: usize) -> Vec<String> {
        let start = self.lines[..=member_index].iter().rposition(|line| !is_tool_line(line.kind)).map_or(0, |index| index + 1);
        self.lines[start..].iter().take_while(|line| is_tool_line(line.kind)).filter_map(|line| line.tool_call_id.clone()).collect()
    }
}
