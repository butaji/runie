use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 50;

#[derive(Debug, Clone)]
pub struct StreamingBuffer {
    stable: Vec<String>,
    tail: String,
    open_fence: Option<String>,
    open_table: bool,
    last_flush: Option<Instant>,
}

impl Default for StreamingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingBuffer {
    pub fn new() -> Self {
        Self {
            stable: Vec::new(),
            tail: String::new(),
            open_fence: None,
            open_table: false,
            last_flush: None,
        }
    }

    pub fn push_delta(&mut self, delta: &str) {
        self.tail.push_str(delta);
        self.resolve();
    }

    pub fn flush(&mut self) -> Vec<String> {
        if self.stable.is_empty() {
            return Vec::new();
        }
        if let Some(last) = self.last_flush {
            if last.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                return Vec::new();
            }
        }
        self.last_flush = Some(Instant::now());
        self.stable.drain(..).collect()
    }

    pub fn force_flush(&mut self) -> Vec<String> {
        self.last_flush = Some(Instant::now());
        self.stable.drain(..).collect()
    }

    pub fn tail(&self) -> &str {
        &self.tail
    }

    pub fn is_stable(&self) -> bool {
        self.tail.is_empty() && self.open_fence.is_none() && !self.open_table
    }

    pub fn reset(&mut self) {
        self.stable.clear();
        self.tail.clear();
        self.open_fence = None;
        self.open_table = false;
        self.last_flush = None;
    }

    fn resolve(&mut self) {
        if self.tail.is_empty() {
            return;
        }

        let current = std::mem::take(&mut self.tail);
        let ends_with_nl = current.ends_with('\n');
        let lines: Vec<&str> = current.split('\n').collect();
        let n = lines.len();

        let mut fence_open = self.open_fence.clone();
        let mut table_open = self.open_table;
        let mut stable_count = 0usize;
        let mut in_open_construct = fence_open.is_some() || table_open;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if in_open_construct {
                if let Some(ref _lang) = fence_open {
                    if trimmed.starts_with("```") {
                        fence_open = None;
                        in_open_construct = false;
                        stable_count = i + 1;
                    }
                    continue;
                }
                if table_open {
                    if trimmed.is_empty() {
                        table_open = false;
                        in_open_construct = false;
                        stable_count = i + 1;
                    }
                    continue;
                }
            }

            if trimmed.is_empty() {
                stable_count = i + 1;
                continue;
            }

            if trimmed.starts_with("```") {
                let lang = trimmed.trim_start_matches("```").trim().to_string();
                fence_open = Some(lang);
                in_open_construct = true;
                continue;
            }

            if is_table_separator(trimmed) {
                table_open = true;
                in_open_construct = true;
                continue;
            }

            stable_count = i + 1;
        }

        self.open_fence = fence_open;
        self.open_table = table_open;

        if stable_count > 0 {
            for &line in lines.iter().take(stable_count) {
                self.stable.push(line.to_string());
            }
        }

        if stable_count < n {
            let remaining = &lines[stable_count..];
            self.tail = remaining.join("\n");
            if ends_with_nl && remaining.len() == 1 && remaining[0].is_empty() {
                self.tail.clear();
            }
        }
    }
}

fn is_table_separator(line: &str) -> bool {
    let stripped = line.trim();
    if !stripped.starts_with('|') || !stripped.ends_with('|') {
        return false;
    }
    let inner = &stripped[1..stripped.len() - 1];
    inner
        .split('|')
        .all(|cell| cell.trim().chars().all(|c| c == '-' || c == ':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_flushes_complete_paragraph() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("Hello, world!\n\n");
        let flushed = buf.force_flush();
        assert_eq!(flushed, vec!["Hello, world!", ""]);
        assert!(buf.tail().is_empty());
        assert!(buf.is_stable());
    }

    #[test]
    fn buffer_holds_incomplete_code_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("Some text.\n```python\nprint('hello')");
        let flushed = buf.force_flush();
        assert_eq!(flushed, vec!["Some text."]);
        assert_eq!(buf.tail(), "```python\nprint('hello')");
        assert!(!buf.is_stable());
    }

    #[test]
    fn buffer_completes_code_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("Some text.\n```python\nprint('hello')\n```");
        let flushed = buf.force_flush();
        assert_eq!(flushed, vec!["Some text.", "```python", "print('hello')", "```"]);
        assert!(buf.tail().is_empty());
        assert!(buf.is_stable());
    }

    #[test]
    fn buffer_batches_deltas() {
        let mut buf = StreamingBuffer::new();
        for i in 0..10 {
            buf.push_delta(&format!("word{} ", i));
        }
        buf.push_delta("\n\n");

        let first = buf.flush();
        assert!(first.is_empty(), "debounce should suppress early flush");

        buf.last_flush = None;

        let flushed = buf.flush();
        assert_eq!(flushed.len(), 2);
        assert!(flushed[0].contains("word0"));
        assert!(flushed[0].contains("word9"));
    }

    #[test]
    fn buffer_tracks_table_open() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("| Header |\n| --- |\n| cell |");
        let flushed = buf.force_flush();
        assert_eq!(flushed, vec!["| Header |"]);
        assert_eq!(buf.tail(), "| --- |\n| cell |");
        assert!(buf.open_table);
    }

    #[test]
    fn buffer_table_ends_at_blank_line() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("| Header |\n| --- |\n| cell |\n\n");
        let flushed = buf.force_flush();
        assert_eq!(
            flushed,
            vec!["| Header |", "| --- |", "| cell |", ""]
        );
        assert!(buf.tail().is_empty());
        assert!(!buf.open_table);
    }

    #[test]
    fn buffer_reset_clears_all() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("hello\n");
        buf.reset();
        assert!(buf.tail().is_empty());
        assert!(buf.stable.is_empty());
        assert!(buf.is_stable());
    }
}
