impl FeedState {
    fn scroll_by(&mut self, delta: i32) {
        if delta == 0 {
            return;
        }
        self.navigation.detach_from_tail();
        if delta.is_negative() {
            self.navigation.scroll_offset = self
                .navigation
                .scroll_offset
                .saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.navigation.scroll_offset =
                self.navigation.scroll_offset.saturating_add(delta as usize);
        }
    }
}
