impl FeedState {
    fn replace_or_append_activity(&mut self, activity: Option<String>) {
        let Some(activity) = activity else {
            return;
        };
        let latest_user = self
            .lines
            .iter()
            .rposition(|line| line.kind == LineKind::User);
        let latest_activity = self
            .lines
            .iter()
            .enumerate()
            .rev()
            .find(|(_, line)| line.kind == LineKind::Activity)
            .map(|(index, _)| index);
        if let Some(index) =
            latest_activity.filter(|index| latest_user.is_none_or(|user_index| *index > user_index))
        {
            self.lines[index].text = activity;
        } else {
            self.append(Line::new(LineKind::Activity, activity));
        }
    }
}
