impl FeedState {
    fn append_to_last_by_kind(&mut self, kind: LineKind, text: String) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) {
            line.text.push_str(&text);
        } else {
            self.append(Line::new(kind, text));
        }
    }
}
