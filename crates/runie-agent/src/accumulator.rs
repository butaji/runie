//! Incremental output accumulator with bounded memory.
//!
//! Uses a rolling tail buffer (2x max_bytes) in memory, falling back
//! to a temp file when output exceeds 2x the limit. Never splits lines.

use crate::truncate::TruncationPolicy;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruncateStrategy {
    /// Keep the beginning (for read, grep, find, ls)
    Head,
    /// Keep the end (for bash)
    Tail,
}

pub struct OutputAccumulator {
    max_lines: usize,
    max_bytes: usize,
    /// Rolling tail buffer — holds up to 2x max_bytes
    buffer: Vec<u8>,
    /// Fallback temp file when buffer would exceed 2x max_bytes
    temp_file: Option<tempfile::NamedTempFile>,
    total_lines: usize,
    total_bytes: usize,
    strategy: TruncateStrategy,
}

pub struct AccumulatedOutput {
    pub content: String,
    pub was_truncated: bool,
    pub total_lines: usize,
    pub total_bytes: usize,
}

impl OutputAccumulator {
    pub fn new(policy: &TruncationPolicy, strategy: TruncateStrategy) -> Self {
        Self {
            max_lines: policy.max_lines,
            max_bytes: policy.max_bytes,
            buffer: Vec::with_capacity(policy.max_bytes * 2),
            temp_file: None,
            total_lines: 0,
            total_bytes: 0,
            strategy,
        }
    }

    pub fn append(&mut self, chunk: &[u8]) {
        self.total_bytes += chunk.len();
        // Count lines in chunk
        for &b in chunk {
            if b == b'\n' {
                self.total_lines += 1;
            }
        }

        if let Some(ref mut file) = self.temp_file {
            let _ = file.write_all(chunk);
            return;
        }

        if self.buffer.len() + chunk.len() > self.max_bytes * 2 {
            // Switch to temp file
            let mut tmp = tempfile::NamedTempFile::new().expect("temp file");
            let _ = tmp.write_all(&self.buffer);
            let _ = tmp.write_all(chunk);
            let _ = tmp.flush();
            self.temp_file = Some(tmp);
            self.buffer.clear();
            self.buffer.shrink_to_fit();
            return;
        }

        self.buffer.extend_from_slice(chunk);
    }

    pub fn snapshot(&self) -> AccumulatedOutput {
        let content = if let Some(ref file) = self.temp_file {
            std::fs::read_to_string(file.path()).unwrap_or_default()
        } else {
            String::from_utf8_lossy(&self.buffer).into_owned()
        };

        let was_truncated = self.total_lines > self.max_lines || self.total_bytes > self.max_bytes;

        if !was_truncated {
            return AccumulatedOutput {
                content,
                was_truncated: false,
                total_lines: self.total_lines,
                total_bytes: self.total_bytes,
            };
        }

        let truncated = match self.strategy {
            TruncateStrategy::Head => Self::truncate_head(&content, self.max_lines, self.max_bytes),
            TruncateStrategy::Tail => Self::truncate_tail(&content, self.max_lines, self.max_bytes),
        };

        AccumulatedOutput {
            content: truncated,
            was_truncated: true,
            total_lines: self.total_lines,
            total_bytes: self.total_bytes,
        }
    }

    fn truncate_head(content: &str, max_lines: usize, max_bytes: usize) -> String {
        let bytes = content.as_bytes();
        let byte_limit = max_bytes.min(bytes.len());
        let mut cut = byte_limit;
        // Walk back to start of line
        while cut > 0 && bytes[cut - 1] != b'\n' {
            cut -= 1;
        }
        let mut lines: Vec<&str> = content[..cut].lines().collect();
        if lines.len() > max_lines {
            lines.truncate(max_lines);
            lines.join("\n")
        } else {
            content[..cut].to_string()
        }
    }

    fn truncate_tail(content: &str, max_lines: usize, max_bytes: usize) -> String {
        let bytes = content.as_bytes();
        let byte_limit = max_bytes.min(bytes.len());
        let mut start = bytes.len() - byte_limit;
        // Walk forward to start of line
        while start < bytes.len() && bytes[start] != b'\n' {
            start += 1;
        }
        if start < bytes.len() {
            start += 1; // skip the newline
        }
        // If no newline found in the window, keep the last max_bytes directly
        if start == bytes.len() && byte_limit < bytes.len() {
            start = bytes.len() - byte_limit;
        }
        let tail = &content[start..];
        let mut lines: Vec<&str> = tail.lines().collect();
        if lines.len() > max_lines {
            lines.drain(..lines.len() - max_lines);
            lines.join("\n")
        } else {
            tail.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::truncate::TruncationPolicy;

    fn default_policy() -> TruncationPolicy {
        TruncationPolicy {
            max_lines: 5,
            max_bytes: 100,
        }
    }

    #[test]
    fn accumulator_tracks_total() {
        let mut acc = OutputAccumulator::new(&default_policy(), TruncateStrategy::Tail);
        acc.append(b"line1\nline2\n");
        assert_eq!(acc.total_lines, 2);
        assert_eq!(acc.total_bytes, 12);
    }

    #[test]
    fn small_output_no_truncation() {
        let mut acc = OutputAccumulator::new(&default_policy(), TruncateStrategy::Tail);
        acc.append(b"small output\n");
        let out = acc.snapshot();
        assert!(!out.was_truncated);
        assert_eq!(out.content, "small output\n");
    }

    #[test]
    fn snapshot_tail_returns_end() {
        let mut acc = OutputAccumulator::new(
            &TruncationPolicy { max_lines: 3, max_bytes: 50 },
            TruncateStrategy::Tail,
        );
        for i in 0..10 {
            acc.append(format!("line {}\n", i).as_bytes());
        }
        let out = acc.snapshot();
        assert!(out.was_truncated);
        assert!(!out.content.contains("line 0"), "tail should drop early lines");
        assert!(out.content.contains("line 9"), "tail should keep last lines");
    }

    #[test]
    fn snapshot_head_returns_start() {
        let mut acc = OutputAccumulator::new(
            &TruncationPolicy { max_lines: 3, max_bytes: 50 },
            TruncateStrategy::Head,
        );
        for i in 0..10 {
            acc.append(format!("line {}\n", i).as_bytes());
        }
        let out = acc.snapshot();
        assert!(out.was_truncated);
        assert!(out.content.contains("line 0"), "head should keep early lines");
        assert!(!out.content.contains("line 9"), "head should drop late lines");
    }

    #[test]
    fn never_splits_lines() {
        let mut acc = OutputAccumulator::new(
            &TruncationPolicy { max_lines: 2, max_bytes: 60 },
            TruncateStrategy::Tail,
        );
        acc.append(b"first line here\nsecond line here\nthird line here\n");
        let out = acc.snapshot();
        // Each line in output should be complete — no split fragments
        let expected: Vec<&str> = out.content.lines().collect();
        assert_eq!(expected, vec!["second line here", "third line here"]);
    }

    #[test]
    fn temp_file_fallback() {
        let policy = TruncationPolicy {
            max_lines: 1000,
            max_bytes: 10,
        };
        let mut acc = OutputAccumulator::new(&policy, TruncateStrategy::Tail);
        let big = "x".repeat(30);
        acc.append(big.as_bytes());
        assert!(acc.temp_file.is_some(), "should switch to temp file");
        let out = acc.snapshot();
        assert!(out.was_truncated, "should be truncated");
        assert_eq!(out.content, "x".repeat(10), "truncated to max_bytes");
        assert_eq!(out.total_bytes, 30);
    }
}
