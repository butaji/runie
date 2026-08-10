impl FeedState {
    fn tool_selection_entries(&self) -> Vec<usize> {
        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                (matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                ) && line.tool_call_id.is_some()
                    && seen.insert(tool_member_key(&self.lines, index)))
                .then_some(index)
            })
            .collect::<Vec<_>>()
    }

}
