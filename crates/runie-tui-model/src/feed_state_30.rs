impl FeedState {
    fn finish_workflow(&mut self, run_id: String, status: String, elapsed_ms: Option<u64>) {
        self.replace_workflow(&run_id, &status, elapsed_ms, 0);
    }
}
