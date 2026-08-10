impl FeedState {
    fn mouse_selection_extend(&mut self, position: CellPosition) {
        if let Some(anchor) = self.navigation.cell_selection_anchor {
            self.navigation.cell_selection = Some(CellSelection {
                anchor,
                head: position,
            });
        }
    }
}
