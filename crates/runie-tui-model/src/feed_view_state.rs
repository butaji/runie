// Transcript append/reset and viewport state transitions.

impl FeedState {
    fn measure_layout(&mut self, content_rows: usize, viewport_rows: usize, anchor_row: Option<usize>) {
        if !self.navigation.autoscroll {
            self.navigation.scroll_offset = self.navigation.scroll_offset.saturating_add_signed(measured_anchor_delta(self.navigation.measured_anchor_row, anchor_row));
        }
        self.navigation.measured_content_rows = content_rows;
        self.navigation.measured_viewport_rows = viewport_rows;
        self.navigation.measured_anchor_row = anchor_row;
    }

    fn append(&mut self, line: Line) {
        if line.kind == LineKind::User { self.navigation.follow_latest_user = true; }
        self.lines.push(line);
        if self.navigation.autoscroll { self.navigation.scroll_offset = self.lines.len(); }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.navigation.facts.clear();
        self.navigation.tool_modes.clear();
        self.navigation.revealed_dense_groups.clear();
        self.navigation.selected_tool_id = None;
        self.navigation.selected_tool_row_id = None;
        self.navigation.selected_entry = None;
        self.navigation.scroll_offset = 0;
        self.navigation.follow_latest_user = false;
    }

    fn scroll_by(&mut self, delta: i32) {
        if delta == 0 { return; }
        self.navigation.detach_from_tail();
        if delta.is_negative() { self.navigation.scroll_offset = self.navigation.scroll_offset.saturating_sub(delta.unsigned_abs() as usize); }
        else { self.navigation.scroll_offset = self.navigation.scroll_offset.saturating_add(delta as usize); }
    }
}
