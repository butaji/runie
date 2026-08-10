impl FeedState {
    fn start_tool(
        &mut self,
        tool_call_id: String,
        header: String,
        activity: Option<String>,
        running: bool,
    ) {
        self.replace_or_append_activity(activity);
        if let Some(tool_name) = self.navigation.facts.tool_name(&tool_call_id) {
            self.navigation
                .tool_modes
                .entry(tool_call_id.clone())
                .or_insert_with(|| default_tool_display_mode(tool_name));
        }
        let kind = if running || header.starts_with("Subagent running:") {
            LineKind::ToolRunning
        } else {
            LineKind::Tool
        };
        let row_id = self.navigation.facts.next_tool_row_id;
        self.navigation.facts.next_tool_row_id = row_id.wrapping_add(1);
        self.append(
            Line::new(kind, header)
                .for_tool(tool_call_id)
                .for_tool_row(row_id),
        );
    }
}
