impl FeedState {
    fn replace_line(&mut self, index: usize, text: String) {
        if let Some(line) = self.lines.get_mut(index) {
            line.text = text;
        }
    }
}
