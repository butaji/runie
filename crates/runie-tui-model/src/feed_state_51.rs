impl FeedState {
    fn dense_group_ids(&self, member_index: usize) -> Vec<String> {
        let start = self.lines[..=member_index]
            .iter()
            .rposition(|line| !is_tool_line(line.kind))
            .map_or(0, |index| index + 1);
        self.lines[start..]
            .iter()
            .take_while(|line| is_tool_line(line.kind))
            .filter_map(|line| line.tool_call_id.clone())
            .collect()
    }

}
