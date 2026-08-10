impl FeedState {
    pub fn snapshot(&self) -> FeedSnapshot {
        let mut snapshot = FeedSnapshot::default();
        self.snapshot_content(&mut snapshot);
        self.snapshot_activity(&mut snapshot);
        self.snapshot_navigation(&mut snapshot);
        snapshot
    }
}
