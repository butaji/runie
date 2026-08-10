// Tool-row lifecycle and transcript-selection projections.

impl FeedState {
    fn replace_tool(&mut self, id: &str, text: String) {
        // Prefer the newest actor-owned live row; compatibility-seeded rows
        // remain a fallback when no opaque row identity is available.
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id) && line.kind.is_tool_header()
        }) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
        }
    }

    fn update_tool(&mut self, id: &str, text: String) {
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id) && line.kind.is_tool_header()
        }) {
            line.text = text;
        }
    }

    fn live_header_mut(&mut self, id: &str) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|line| {
            line.tool_row_id.is_some()
                && line.is_tool_row_active()
                && line.tool_call_id.as_deref() == Some(id)
                && line.kind.is_tool_header()
        })
    }

    fn mark_tool_error(&mut self, id: &str) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id) && line.kind.is_tool_header()
        }) {
            line.kind = LineKind::ToolError;
        }
    }

    fn selectable_entries(&self) -> Vec<usize> {
        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let selectable = match line.kind {
                    kind if kind.is_tool_header() => line
                        .tool_call_id
                        .as_ref()
                        .is_none_or(|_| seen.insert(tool_member_key(&self.lines, index))),
                    kind if kind.is_selectable_transcript() => true,
                    _ => false,
                };
                selectable.then_some(index)
            })
            .collect()
    }
}
