impl FeedState {
    fn replace_last_by_kind(&mut self, kind: LineKind, text: String) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) {
            line.text = text;
        }
    }
}
