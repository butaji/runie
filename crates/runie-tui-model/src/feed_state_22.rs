impl FeedState {
    fn clear_cell_selection(&mut self) {
        self.navigation.cell_selection_anchor = None;
        self.navigation.cell_selection = None;
    }
}
