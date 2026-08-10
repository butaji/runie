impl FeedState {
    fn append(&mut self, line: Line) {
        if line.kind == LineKind::User {
            self.navigation.follow_latest_user = true;
        }
        self.lines.push(line);
        if self.navigation.autoscroll {
            self.navigation.scroll_offset = self.lines.len();
        }
    }
}
