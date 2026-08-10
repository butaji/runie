impl FeedState {
    fn live_header_mut(&mut self, id: &str) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|line| {
            line.tool_row_id.is_some()
                && line.is_tool_row_active()
                && line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        })
    }
}
