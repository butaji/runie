impl FeedState {
    fn reduce_navigation(&mut self, message: ScrollbackMsg) {
        match message {
            ScrollbackMsg::ToggleToolMode(id) => self.toggle_tool_mode(&id),
            ScrollbackMsg::SelectRange { anchor, head } => self.select_range(anchor, head),
            ScrollbackMsg::ClearSelection => self.clear_selection(),
            ScrollbackMsg::MouseSelectionStart(position) => self.mouse_selection_start(position),
            ScrollbackMsg::MouseSelectionExtend(position) => self.mouse_selection_extend(position),
            ScrollbackMsg::MouseSelectionCommit => self.navigation.cell_selection_anchor = None,
            ScrollbackMsg::ClearCellSelection => self.clear_cell_selection(),
            ScrollbackMsg::RequestCopySelection => {
                self.navigation.copy_selection = self.navigation.cell_selection
            }
            ScrollbackMsg::ClearCopyRequest => self.navigation.copy_selection = None,
            ScrollbackMsg::SelectNextTool => self.select_tool(1),
            ScrollbackMsg::SelectPreviousTool => self.select_tool(-1),
            ScrollbackMsg::SelectNextEntry => self.select_entry(1),
            ScrollbackMsg::SelectPreviousEntry => self.select_entry(-1),
            ScrollbackMsg::ScrollBy(delta) => self.scroll_by(delta),
            ScrollbackMsg::LayoutMeasured {
                content_rows,
                viewport_rows,
                anchor_row,
            } => self.measure_layout(content_rows, viewport_rows, anchor_row),
            ScrollbackMsg::RevealLatest => self.navigation.reveal_latest(self.lines.len()),
            ScrollbackMsg::MarkToolError(id) => self.mark_tool_error(&id),
            ScrollbackMsg::ReplaceLine(index, text) => self.replace_line(index, text),
            ScrollbackMsg::ReplaceLastByKind(kind, text) => self.replace_last_by_kind(kind, text),
            ScrollbackMsg::AppendToLastByKind(kind, text) => {
                self.append_to_last_by_kind(kind, text)
            }
            ScrollbackMsg::SetToolName(id, name) => {
                self.navigation.facts.tools.entry(id).or_default().name = Some(name);
            }
            ScrollbackMsg::SetToolMode(id, mode) => self.set_tool_mode(id, mode),
            _ => unreachable!("lifecycle messages handled before core reduction"),
        }
    }
}
