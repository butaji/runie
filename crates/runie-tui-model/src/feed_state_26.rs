impl FeedState {
    fn finalize_assistant(
        &mut self,
        has_reasoning: bool,
        reasoning_expanded: bool,
        summary: String,
        settled_no_tool_phase: bool,
    ) {
        self.navigation.facts.settled_no_tool_phase = settled_no_tool_phase;
        if !has_reasoning || reasoning_expanded {
            self.lines
                .retain(|line| line.kind != LineKind::ThinkingStatus);
            return;
        }
        if let Some(line) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| line.kind == LineKind::ThinkingStatus)
        {
            line.kind = LineKind::TurnSummary;
            line.text = summary;
            self.lines.retain(|line| line.kind != LineKind::Reasoning);
        }
    }
}
