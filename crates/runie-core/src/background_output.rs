use super::BackgroundOutput;

impl BackgroundOutput {
    pub fn search_lines(&self, query: &str) -> Vec<String> {
        self.text
            .lines()
            .filter(|line| line.contains(query))
            .map(str::to_owned)
            .collect()
    }

    pub fn preview_terminal_line(&self) -> String {
        self.preview
            .clone()
            .unwrap_or_else(|| "(no output preview)".into())
    }

    pub fn facts_terminal_line(&self) -> String {
        format!(
            "job={} status={:?} command={:?} exit={:?} lines={} bytes={} truncated={} preview={:?}",
            self.job_id,
            self.status,
            self.command,
            self.exit_code,
            self.output_lines,
            self.output_bytes,
            self.truncated,
            self.preview,
        )
    }
}
