impl FeedState {
    fn reveal_dense_group(&mut self, tool_id: &str) {
        let Some(member_index) = self
            .lines
            .iter()
            .position(|line| line.tool_call_id.as_deref() == Some(tool_id))
        else {
            return;
        };
        let ids = self.dense_group_ids(member_index);
        if ids.len() > GROK_GROUP_MAX_VISIBLE {
            self.navigation.revealed_dense_groups.insert(ids[0].clone());
            self.navigation.selected_entry = Some(member_index);
            self.navigation.center_revealed_entry = true;
        }
    }

}
