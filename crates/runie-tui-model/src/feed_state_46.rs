impl FeedState {
    fn selectable_entries(&self) -> Vec<usize> {
        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let selectable = match line.kind {
                    kind if kind.is_tool_header() => line
                        .tool_call_id
                        .as_ref()
                        .is_none_or(|_| seen.insert(tool_member_key(&self.lines, index))),
                    kind if kind.is_selectable_transcript() => true,
                    _ => false,
                };
                selectable.then_some(index)
            })
            .collect()
    }
}
