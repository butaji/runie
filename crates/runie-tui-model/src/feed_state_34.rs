impl FeedState {
    fn settle_tool_mode(&mut self, tool_call_id: &str, mode_key: &str) {
        let Some(name) = self.navigation.facts.tool_name(tool_call_id).map(str::to_owned) else {
            return;
        };
        if matches!(name.as_str(), "read" | "read_file") {
            self.navigation
                .tool_modes
                .insert(mode_key.to_owned(), ToolDisplayMode::Collapsed);
            self.navigation
                .tool_modes
                .insert(tool_call_id.to_owned(), ToolDisplayMode::Collapsed);
        } else if matches!(name.as_str(), "bash" | "shell" | "exec" | "run")
            && self
                .navigation
                .tool_modes
                .get(mode_key)
                .or_else(|| self.navigation.tool_modes.get(tool_call_id))
                == Some(&ToolDisplayMode::Truncated)
        {
            self.navigation
                .tool_modes
                .insert(mode_key.to_owned(), ToolDisplayMode::Expanded);
            self.navigation
                .tool_modes
                .insert(tool_call_id.to_owned(), ToolDisplayMode::Expanded);
        }
    }
}
