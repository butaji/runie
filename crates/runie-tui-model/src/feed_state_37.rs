impl FeedState {
    fn clear(&mut self) {
        self.lines.clear();
        self.navigation.facts.clear();
        self.navigation.tool_modes.clear();
        self.navigation.revealed_dense_groups.clear();
        self.navigation.selected_tool_id = None;
        self.navigation.selected_tool_row_id = None;
        self.navigation.selected_entry = None;
        self.navigation.scroll_offset = 0;
        self.navigation.follow_latest_user = false;
    }
}
