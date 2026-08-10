impl FeedState {
    fn remove_empty_after(&mut self, kind: LineKind) {
        if let Some(index) = self.lines.iter().position(|line| line.kind == kind) {
            if self
                .lines
                .get(index + 1)
                .is_some_and(|line| line.text.is_empty())
            {
                self.lines.remove(index + 1);
            }
        }
    }
}
