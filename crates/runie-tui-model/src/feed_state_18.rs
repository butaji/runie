impl FeedState {
    fn select_range(&mut self, anchor: usize, head: usize) {
        self.navigation.selection_anchor = Some(anchor);
        self.navigation.selection_head = Some(head);
    }
}
