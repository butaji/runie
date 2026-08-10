impl FeedState {
    fn snapshot_content(&self, snapshot: &mut FeedSnapshot) {
        snapshot.lines = self.lines.clone();
        snapshot.facts = FeedFacts::from(&self.navigation);
        snapshot.tool_blocks = project_tool_blocks(
            &self.lines,
            &self.navigation.facts.tools,
            &self.navigation.tool_modes,
        );
    }
}
