impl FeedState {
    fn normalize_activity_spacing(&mut self) {
        let Some(index) = self
            .lines
            .iter()
            .position(|line| line.kind == LineKind::Activity)
        else {
            return;
        };
        self.lines
            .retain(|line| !(line.kind == LineKind::System && line.text.is_empty()));
        self.lines
            .insert(index + 1, Line::new(LineKind::Separator, ""));
    }
}
