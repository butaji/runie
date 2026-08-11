#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpStatusRow {
    pub transport: String,
    pub index: usize,
    pub identity: String,
    pub status: String,
}

impl McpStatusRow {
    pub fn terminal_line(&self) -> String {
        format!(
            "{}[{}] {} status={}",
            self.transport, self.index, self.identity, self.status
        )
    }
}
