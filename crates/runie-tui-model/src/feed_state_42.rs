impl FeedState {
    fn mark_tool_error(&mut self, id: &str) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.kind = LineKind::ToolError;
        }
    }
}
