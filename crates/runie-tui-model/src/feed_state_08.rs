impl FeedState {
    fn reduce_lifecycle(&mut self, message: ScrollbackMsg) -> Result<(), ScrollbackMsg> {
        let message = match self.reduce_tool(message) {
            Ok(()) => return Ok(()),
            Err(message) => message,
        };
        if let ScrollbackMsg::FinalizeAssistant {
            has_reasoning,
            reasoning_expanded,
            summary,
            settled_no_tool_phase,
        } = message
        {
            return self.reduce_finalize(
                has_reasoning,
                reasoning_expanded,
                summary,
                settled_no_tool_phase,
            );
        }
        self.reduce_workflow(message)
    }
}
