impl FeedState {
    fn tool_output_suffix_matches(&self, id: &str, output: &[(LineKind, String)]) -> bool {
        if output.is_empty() || self.lines.len() < output.len() {
            return false;
        }
        let existing: Vec<&str> = self
            .lines
            .iter()
            .filter(|line| line.tool_call_id.as_deref() == Some(id))
            .map(|line| line.text.as_str())
            .collect();
        output
            .iter()
            .all(|(_kind, expected)| existing.contains(&expected.as_str()))
    }
}
