impl FeedState {
    fn finish_tool(
        &mut self,
        tool_call_id: String,
        header: String,
        activity: Option<String>,
        output: Vec<(LineKind, String)>,
    ) {
        let mode_key = self.tool_mode_key(&tool_call_id);
        self.replace_tool(&tool_call_id, header);
        self.settle_tool_mode(&tool_call_id, &mode_key);
        if !self.tool_output_suffix_matches(&tool_call_id, &output) {
            for (kind, text) in output {
                self.append(Line::new(kind, text).for_tool(&tool_call_id));
            }
        }
        self.replace_or_append_activity(activity);
    }
}
