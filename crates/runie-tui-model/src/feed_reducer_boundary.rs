// Ordered event-reducer pipeline: lifecycle, content, then navigation.

impl FeedState {
    pub fn reduce(&mut self, message: ScrollbackMsg) {
        macro_rules! reduce_stage {
            ($state:expr, $message:expr, $($stage:ident),+ $(,)?) => {{
                let mut message = $message;
                $( message = match $state.$stage(message) { Ok(()) => return, Err(message) => message }; )+
                $state.reduce_navigation(message);
            }};
        }
        reduce_stage!(self, message, reduce_lifecycle, reduce_content);
    }
    fn reduce_navigation(&mut self, message: ScrollbackMsg) {
        match message {
            ScrollbackMsg::ToggleToolMode(id) => self.toggle_tool_mode(&id),
            ScrollbackMsg::SelectRange { anchor, head } => self.select_range(anchor, head),
            ScrollbackMsg::ClearSelection => self.clear_selection(),
            ScrollbackMsg::MouseSelectionStart(position) => self.mouse_selection_start(position),
            ScrollbackMsg::MouseSelectionExtend(position) => self.mouse_selection_extend(position),
            ScrollbackMsg::MouseSelectionCommit => self.navigation.cell_selection_anchor = None,
            ScrollbackMsg::ClearCellSelection => self.clear_cell_selection(),
            ScrollbackMsg::RequestCopySelection => self.navigation.copy_selection = self.navigation.cell_selection,
            ScrollbackMsg::ClearCopyRequest => self.navigation.copy_selection = None,
            ScrollbackMsg::SelectNextTool => self.select_tool(1), ScrollbackMsg::SelectPreviousTool => self.select_tool(-1),
            ScrollbackMsg::SelectNextEntry => self.select_entry(1), ScrollbackMsg::SelectPreviousEntry => self.select_entry(-1),
            ScrollbackMsg::ScrollBy(delta) => self.scroll_by(delta),
            ScrollbackMsg::LayoutMeasured { content_rows, viewport_rows, anchor_row } => self.measure_layout(content_rows, viewport_rows, anchor_row),
            ScrollbackMsg::RevealLatest => self.navigation.reveal_latest(self.lines.len()),
            ScrollbackMsg::MarkToolError(id) => self.mark_tool_error(&id),
            ScrollbackMsg::ReplaceLine(index, text) => self.replace_line(index, text),
            ScrollbackMsg::ReplaceLastByKind(kind, text) => self.replace_last_by_kind(kind, text),
            ScrollbackMsg::AppendToLastByKind(kind, text) => self.append_to_last_by_kind(kind, text),
            ScrollbackMsg::SetToolName(id, name) => self.set_tool_name(id, name), ScrollbackMsg::SetToolMode(id, mode) => self.set_tool_mode(id, mode),
            _ => unreachable!("lifecycle messages handled before core reduction"),
        }
    }
    fn reduce_lifecycle(&mut self, message: ScrollbackMsg) -> Result<(), ScrollbackMsg> {
        let message = match self.reduce_tool(message) { Ok(()) => return Ok(()), Err(message) => message };
        if let ScrollbackMsg::FinalizeAssistant { has_reasoning, reasoning_expanded, summary, settled_no_tool_phase } = message {
            return self.reduce_finalize(has_reasoning, reasoning_expanded, summary, settled_no_tool_phase);
        }
        self.reduce_workflow(message)
    }
    fn reduce_finalize(&mut self, has_reasoning: bool, reasoning_expanded: bool, summary: String, settled_no_tool_phase: bool) -> Result<(), ScrollbackMsg> {
        self.finalize_assistant(has_reasoning, reasoning_expanded, summary, settled_no_tool_phase); Ok(())
    }
}
