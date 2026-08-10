// Immutable feed snapshot assembly and projection schema.

macro_rules! project_navigation {
    ($snapshot:expr, $navigation:expr, $selected_member_index:expr;
        $($field:ident),*;
        clone $($clone_field:ident),* $(,)?) => {
        $( $snapshot.$clone_field = $navigation.$clone_field.clone(); )*
        $( $snapshot.$field = project_navigation!(@value $navigation, $selected_member_index; $field); )*
    };
    (@value $navigation:expr, $selected_member_index:expr; selected_member_index) => { $selected_member_index };
    (@value $navigation:expr, $selected_member_index:expr; $field:ident) => { $navigation.$field };
}

impl FeedState {
    pub fn snapshot(&self) -> FeedSnapshot {
        let mut snapshot = FeedSnapshot::default();
        self.snapshot_content(&mut snapshot);
        self.snapshot_activity(&mut snapshot);
        self.snapshot_navigation(&mut snapshot);
        snapshot
    }

    fn snapshot_content(&self, snapshot: &mut FeedSnapshot) {
        snapshot.lines = self.lines.clone();
        snapshot.facts = FeedFacts::from(&self.navigation);
        snapshot.tool_blocks = project_tool_blocks(&self.lines, &self.navigation.facts.tools, &self.navigation.tool_modes);
    }

    fn snapshot_activity(&self, _snapshot: &mut FeedSnapshot) {}

    fn snapshot_navigation(&self, snapshot: &mut FeedSnapshot) {
        project_navigation!(snapshot, self.navigation, self.selected_member_index();
            autoscroll, scroll_offset, reasoning_expanded, activity_expanded,
            center_revealed_entry, follow_latest_user, selected_tool_row_id,
            selected_entry, selection_anchor, selection_head, cell_selection,
            copy_selection, selected_member_index, theme, animation_frame,
            measured_content_rows, measured_viewport_rows, measured_anchor_row
            ; clone prompt_timestamp, revealed_dense_groups, selected_tool_id
        );
    }

    fn selected_member_index(&self) -> Option<usize> {
        let entry = self.navigation.selected_entry?;
        logical_tool_member_index_at(&self.lines, entry)
    }
}
