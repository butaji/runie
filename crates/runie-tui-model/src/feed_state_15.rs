impl FeedState {
    fn reset_activity(&mut self) {
        self.navigation.facts.reset_activity();
    }
}
