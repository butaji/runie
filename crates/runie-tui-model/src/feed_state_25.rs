impl FeedState {
    fn measure_layout(
        &mut self,
        content_rows: usize,
        viewport_rows: usize,
        anchor_row: Option<usize>,
    ) {
        if !self.navigation.autoscroll {
            self.navigation.scroll_offset =
                self.navigation
                    .scroll_offset
                    .saturating_add_signed(measured_anchor_delta(
                        self.navigation.measured_anchor_row,
                        anchor_row,
                    ));
        }
        self.navigation.measured_content_rows = content_rows;
        self.navigation.measured_viewport_rows = viewport_rows;
        self.navigation.measured_anchor_row = anchor_row;
    }
}
