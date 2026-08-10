impl FeedState {
    fn reduce_finalize(
        &mut self,
        has_reasoning: bool,
        reasoning_expanded: bool,
        summary: String,
        settled_no_tool_phase: bool,
    ) -> Result<(), ScrollbackMsg> {
        self.finalize_assistant(
            has_reasoning,
            reasoning_expanded,
            summary,
            settled_no_tool_phase,
        );
        Ok(())
    }
}
