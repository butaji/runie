impl FeedState {
    fn normalize_assistants(&mut self) {
        for line in &mut self.lines {
            if line.kind == LineKind::Assistant && !line.text.is_empty() {
                line.kind = LineKind::CompletedAssistant;
            }
        }
    }
}
