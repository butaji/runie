impl FeedState {
    fn mouse_selection_start(&mut self, position: CellPosition) {
        self.navigation.cell_selection_anchor = Some(position);
        self.navigation.cell_selection = None;
    }
}
