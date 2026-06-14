#[derive(Debug, Clone)]
pub struct TruncationSection {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationSection {
    fn default() -> Self {
        Self {
            max_lines: 1000,
            max_bytes: 100_000,
        }
    }
}
