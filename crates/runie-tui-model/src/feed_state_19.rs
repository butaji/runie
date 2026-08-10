impl FeedState {
    fn clear_selection(&mut self) {
        self.navigation.selection_anchor = None;
        self.navigation.selection_head = None;
    }
}
