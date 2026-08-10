impl FeedState {
    fn start_activity_tool(&mut self, name: &str) {
        match classify_activity_tool(name) {
            Some(ActivityKind::Dir) => self.navigation.facts.activity_dirs += 1,
            Some(ActivityKind::File) => self.navigation.facts.activity_files += 1,
            Some(ActivityKind::Command) => self.navigation.facts.activity_commands += 1,
            Some(ActivityKind::Subagent) => self.navigation.facts.activity_subagents += 1,
            None => {}
        }
    }
}
