impl FeedState {
    fn update_tool_output(&mut self, id: String, header: Option<String>, output: Vec<String>) {
        if let Some(header) = header {
            self.update_tool(&id, header);
        }
        for text in output {
            self.append(Line::new(LineKind::ToolOutput, text).for_tool(&id));
        }
    }
}
