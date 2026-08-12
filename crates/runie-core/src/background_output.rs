use super::BackgroundOutput;

impl BackgroundOutput {
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
