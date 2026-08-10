impl FeedState {
    fn toggle_tool_mode(&mut self, id: &str) {
        let read_card = self
            .navigation
            .facts
            .tool_name(id)
            .is_some_and(|name| matches!(name, "read" | "read_file"));
        let running_generic_card = project_tool_blocks(
            &self.lines,
            &self.navigation.facts.tools,
            &self.navigation.tool_modes,
        )
        .iter()
        .any(|block| {
            block.tool_call_id == id && block.is_running && block.kind == ToolCardKind::Generic
        });
        let mode = self
            .navigation
            .tool_modes
            .get(id)
            .copied()
            .unwrap_or(ToolDisplayMode::Expanded);
        let next = match mode {
            ToolDisplayMode::Collapsed if read_card || running_generic_card => {
                ToolDisplayMode::Truncated
            }
            ToolDisplayMode::Collapsed => ToolDisplayMode::Expanded,
            ToolDisplayMode::Truncated if running_generic_card => ToolDisplayMode::Expanded,
            ToolDisplayMode::Truncated => ToolDisplayMode::Collapsed,
            ToolDisplayMode::Expanded if running_generic_card => ToolDisplayMode::Truncated,
            ToolDisplayMode::Expanded => ToolDisplayMode::Collapsed,
        };
        self.navigation.tool_modes.insert(id.to_owned(), next);
    }

}
