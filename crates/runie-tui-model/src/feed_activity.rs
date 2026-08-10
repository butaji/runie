// Activity facts and transcript presentation transitions.

impl FeedState {
    fn finish_activity_tool(&mut self, is_error: bool) {
        if is_error { self.navigation.facts.activity_failures += 1; }
    }
    fn start_activity_tool(&mut self, name: &str) {
        match classify_activity_tool(name) {
            Some(ActivityKind::Dir) => self.navigation.facts.activity_dirs += 1,
            Some(ActivityKind::File) => self.navigation.facts.activity_files += 1,
            Some(ActivityKind::Command) => self.navigation.facts.activity_commands += 1,
            Some(ActivityKind::Subagent) => self.navigation.facts.activity_subagents += 1,
            None => {}
        }
    }
    fn replace_or_append_activity(&mut self, activity: Option<String>) {
        let Some(activity) = activity else { return; };
        let latest_user = self.lines.iter().rposition(|line| line.kind == LineKind::User);
        let latest_activity = self.lines.iter().enumerate().rev().find(|(_, line)| line.kind == LineKind::Activity).map(|(index, _)| index);
        if let Some(index) = latest_activity.filter(|index| latest_user.is_none_or(|user| *index > user)) { self.lines[index].text = activity; }
        else { self.append(Line::new(LineKind::Activity, activity)); }
    }
    fn normalize_activity_spacing(&mut self) {
        let Some(index) = self.lines.iter().position(|line| line.kind == LineKind::Activity) else { return; };
        self.lines.retain(|line| !(line.kind == LineKind::System && line.text.is_empty()));
        self.lines.insert(index + 1, Line::new(LineKind::Separator, ""));
    }
}
