// Logical range and terminal-cell selection state transitions.

impl FeedState {
    fn select_range(&mut self, anchor: usize, head: usize) {
        self.navigation.selection_anchor = Some(anchor);
        self.navigation.selection_head = Some(head);
    }
    fn clear_selection(&mut self) {
        self.navigation.selection_anchor = None;
        self.navigation.selection_head = None;
    }
    fn mouse_selection_start(&mut self, position: CellPosition) {
        self.navigation.cell_selection_anchor = Some(position);
        self.navigation.cell_selection = None;
    }
    fn mouse_selection_extend(&mut self, position: CellPosition) {
        if let Some(anchor) = self.navigation.cell_selection_anchor {
            self.navigation.cell_selection = Some(CellSelection { anchor, head: position });
        }
    }
    fn clear_cell_selection(&mut self) {
        self.navigation.cell_selection_anchor = None;
        self.navigation.cell_selection = None;
    }
}
