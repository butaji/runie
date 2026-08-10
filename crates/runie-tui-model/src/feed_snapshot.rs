impl FeedSnapshot {
    /// Whether the feed contains a live tool row requiring animation ticks.
    pub fn animation_demand(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.kind == LineKind::ToolRunning)
    }
}
