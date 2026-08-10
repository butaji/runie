impl FeedState {
    fn set_tool_mode(&mut self, id: String, mode: ToolDisplayMode) {
        let row_id = self
            .lines
            .iter()
            .rev()
            .find(|line| line.tool_call_id.as_deref() == Some(id.as_str()))
            .and_then(|line| line.tool_row_id);
        if let Some(row_id) = row_id {
            self.navigation
                .tool_modes
                .insert(format!("#row:{row_id}"), mode);
        }
        self.navigation.tool_modes.insert(id, mode);
    }
}
