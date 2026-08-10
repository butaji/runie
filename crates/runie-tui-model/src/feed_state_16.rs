impl FeedState {
    fn finish_activity_tool(&mut self, is_error: bool) {
        if is_error {
            self.navigation.facts.activity_failures += 1;
        }
    }
}
