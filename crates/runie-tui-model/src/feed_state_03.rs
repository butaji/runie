impl FeedState {
    fn snapshot_navigation(&self, snapshot: &mut FeedSnapshot) {
        snapshot.autoscroll = self.navigation.autoscroll;
        snapshot.scroll_offset = self.navigation.scroll_offset;
        snapshot.reasoning_expanded = self.navigation.reasoning_expanded;
        snapshot.activity_expanded = self.navigation.activity_expanded;
        snapshot.prompt_timestamp = self.navigation.prompt_timestamp.clone();
        snapshot.revealed_dense_groups = self.navigation.revealed_dense_groups.clone();
        snapshot.center_revealed_entry = self.navigation.center_revealed_entry;
        snapshot.follow_latest_user = self.navigation.follow_latest_user;
        snapshot.selected_tool_id = self.navigation.selected_tool_id.clone();
        snapshot.selected_tool_row_id = self.navigation.selected_tool_row_id;
        snapshot.selected_entry = self.navigation.selected_entry;
        snapshot.selection_anchor = self.navigation.selection_anchor;
        snapshot.selection_head = self.navigation.selection_head;
        snapshot.cell_selection = self.navigation.cell_selection;
        snapshot.copy_selection = self.navigation.copy_selection;
        snapshot.selected_member_index = self.selected_member_index();
        snapshot.theme = self.navigation.theme;
        snapshot.animation_frame = self.navigation.animation_frame;
        snapshot.measured_content_rows = self.navigation.measured_content_rows;
        snapshot.measured_viewport_rows = self.navigation.measured_viewport_rows;
        snapshot.measured_anchor_row = self.navigation.measured_anchor_row;
    }
}
