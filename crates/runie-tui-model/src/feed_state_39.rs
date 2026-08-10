impl FeedState {
    fn update_tool(&mut self, id: &str, text: String) {
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && line.kind.is_tool_header()
        }) {
            line.text = text;
        }
    }
}
