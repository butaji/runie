impl FeedState {
    fn selected_member_index(&self) -> Option<usize> {
        let entry = self.navigation.selected_entry?;
        logical_tool_member_index_at(&self.lines, entry)
    }
}
