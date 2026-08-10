// Purely local transcript line mutations used by event reducers.

impl FeedState {
    fn replace_line(&mut self, index: usize, text: String) {
        if let Some(line) = self.lines.get_mut(index) { line.text = text; }
    }
    fn replace_last_by_kind(&mut self, kind: LineKind, text: String) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) { line.text = text; }
    }
    fn append_to_last_by_kind(&mut self, kind: LineKind, text: String) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) { line.text.push_str(&text); }
        else { self.append(Line::new(kind, text)); }
    }
    fn remove_empty_after(&mut self, kind: LineKind) {
        if let Some(index) = self.lines.iter().position(|line| line.kind == kind) {
            if self.lines.get(index + 1).is_some_and(|line| line.text.is_empty()) {
                self.lines.remove(index + 1);
            }
        }
    }
}
