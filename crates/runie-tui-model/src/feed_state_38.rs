impl FeedState {
    fn replace_tool(&mut self, id: &str, text: String) {
        // Provider call IDs are not guaranteed to be unique across replayed
        // or concurrent lifecycle fragments. Prefer the newest actor-owned
        // live row, exactly as the event stream's row identity requires;
        // falling back to a settled row is only for compatibility-seeded
        // transcripts that have no opaque row identity.
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && line.kind.is_tool_header()
        }) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
        }
    }
}
