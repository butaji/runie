// Tool lifecycle reduction: start, finish, display mode, and output identity.

impl FeedState {
    fn finish_tool(&mut self, tool_call_id: String, header: String, activity: Option<String>, output: Vec<(LineKind, String)>) {
        let mode_key = self.tool_mode_key(&tool_call_id);
        self.replace_tool(&tool_call_id, header);
        self.settle_tool_mode(&tool_call_id, &mode_key);
        if !self.tool_output_suffix_matches(&tool_call_id, &output) {
            for (kind, text) in output { self.append(Line::new(kind, text).for_tool(&tool_call_id)); }
        }
        self.replace_or_append_activity(activity);
    }
    fn tool_mode_key(&self, tool_call_id: &str) -> String {
        self.lines.iter().rev().find(|line| line.is_tool_row_active() && line.tool_call_id.as_deref() == Some(tool_call_id))
            .and_then(|line| line.tool_row_id).map_or_else(|| tool_call_id.to_owned(), |row_id| format!("#row:{row_id}"))
    }
    fn settle_tool_mode(&mut self, tool_call_id: &str, mode_key: &str) {
        let Some(name) = self.navigation.facts.tool_name(tool_call_id).map(str::to_owned) else { return; };
        let next = if matches!(name.as_str(), "read" | "read_file") { Some(ToolDisplayMode::Collapsed) }
            else if matches!(name.as_str(), "bash" | "shell" | "exec" | "run") && self.navigation.tool_modes.get(mode_key).or_else(|| self.navigation.tool_modes.get(tool_call_id)) == Some(&ToolDisplayMode::Truncated) { Some(ToolDisplayMode::Expanded) }
            else { None };
        if let Some(next) = next { for key in [mode_key, tool_call_id] { self.navigation.tool_modes.insert(key.to_owned(), next); } }
    }
    fn start_tool(&mut self, tool_call_id: String, header: String, activity: Option<String>, running: bool) {
        self.replace_or_append_activity(activity);
        if let Some(tool_name) = self.navigation.facts.tool_name(&tool_call_id) { self.navigation.tool_modes.entry(tool_call_id.clone()).or_insert_with(|| default_tool_display_mode(tool_name)); }
        let kind = if running || header.starts_with("Subagent running:") { LineKind::ToolRunning } else { LineKind::Tool };
        let row_id = self.navigation.facts.next_tool_row_id;
        self.navigation.facts.next_tool_row_id = row_id.wrapping_add(1);
        self.append(Line::new(kind, header).for_tool(tool_call_id).for_tool_row(row_id));
    }
    fn tool_output_suffix_matches(&self, id: &str, output: &[(LineKind, String)]) -> bool {
        if output.is_empty() || self.lines.len() < output.len() { return false; }
        let existing: Vec<&str> = self.lines.iter().filter(|line| line.tool_call_id.as_deref() == Some(id)).map(|line| line.text.as_str()).collect();
        output.iter().all(|(_kind, expected)| existing.contains(&expected.as_str()))
    }
}
