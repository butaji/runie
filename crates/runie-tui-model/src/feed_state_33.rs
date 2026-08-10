impl FeedState {
    fn tool_mode_key(&self, tool_call_id: &str) -> String {
        self.lines
            .iter()
            .rev()
            .find(|line| {
                line.is_tool_row_active() && line.tool_call_id.as_deref() == Some(tool_call_id)
            })
            .and_then(|line| line.tool_row_id)
            .map_or_else(
                || tool_call_id.to_owned(),
                |row_id| format!("#row:{row_id}"),
            )
    }
}
