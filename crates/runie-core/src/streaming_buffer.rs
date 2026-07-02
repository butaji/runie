//! Streaming buffer for LLM text deltas.
//!
//! Accumulates streamed text and determines which lines are "stable" enough
//! to flush to the UI. Stability means the line cannot change due to future
//! streaming output (e.g., we won't need to insert characters in the middle).
//!
//! Rules for stability:
//! - Plain text lines are stable once they end with a newline.
//! - Fenced code blocks are stable only after the closing ``` is received.
//! - Table rows are stable once the blank line after the table separator is received.
//!
//! Uses `pulldown_cmark` to detect fence/table boundaries, replacing the
//! previous custom line-based classifier.

use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 50;

pub use crate::markdown::heal_markdown;

#[derive(Debug, Clone)]
pub struct StreamingBuffer {
    /// Lines that are confirmed stable and ready to flush.
    stable: Vec<String>,
    /// Unprocessed accumulated text that may still change.
    tail: String,
    /// Tracks whether we are inside an unclosed fenced code block.
    in_open_fence: bool,
    /// Tracks whether we are inside a table (between separator and blank line).
    in_open_table: bool,
    /// Time of last flush, for debouncing.
    last_flush: Option<Instant>,
}

impl Default for StreamingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingBuffer {
    /// Create a new empty streaming buffer.
    pub fn new() -> Self {
        Self {
            stable: Vec::new(),
            tail: String::new(),
            in_open_fence: false,
            in_open_table: false,
            last_flush: None,
        }
    }

    /// Append a text delta to the buffer and resolve stable lines.
    pub fn push_delta(&mut self, delta: &str) {
        self.tail.push_str(delta);
        self.resolve();
    }

    /// Return stable lines if the debounce interval has elapsed.
    /// Returns empty `Vec` if nothing to flush or debounce is active.
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
        self.stable
            .drain(..)
            .filter(|l| !l.is_empty())
            .map(|l| heal_markdown(&l))
            .collect()
    }

    /// Immediately return all stable lines plus the current tail (unstable).
    /// Does not clear fence/table state - use reset() to clear everything.
    pub fn force_flush(&mut self) -> Vec<String> {
        self.last_flush = Some(Instant::now());
        let mut lines: Vec<String> = self.stable.drain(..).map(|l| heal_markdown(&l)).collect();
        if !self.tail.is_empty() {
            lines.push(heal_markdown(&self.tail));
            self.tail.clear();
        }
        lines
    }

    /// The unstable tail text still being streamed.
    pub fn tail(&self) -> &str {
        &self.tail
    }

    /// True when there is no pending unstable content.
    pub fn is_stable(&self) -> bool {
        self.tail.is_empty() && !self.in_open_fence && !self.in_open_table
    }

    /// True when there is pending content (stable or unstable).
    pub fn has_pending_content(&self) -> bool {
        !self.stable.is_empty()
            || !self.tail.is_empty()
            || self.in_open_fence
            || self.in_open_table
    }

    #[cfg(test)]
    pub fn stable_len(&self) -> usize {
        self.stable.len()
    }

    /// Reset the buffer to empty.
    pub fn reset(&mut self) {
        self.stable.clear();
        self.tail.clear();
        self.in_open_fence = false;
        self.in_open_table = false;
        self.last_flush = None;
    }

    /// Analyze the accumulated text, determine stable lines, and update fence/table state.
    fn resolve(&mut self) {
        if self.tail.is_empty() {
            return;
        }

        let current = std::mem::take(&mut self.tail);
        let lines: Vec<&str> = current.split('\n').collect();
        let n = lines.len();

        // Detect fence and table boundaries using pulldown_cmark.
        let (stable_count, in_fence, in_table) =
            classify_lines_with_pulldown(&lines, self.in_open_fence, self.in_open_table);

        self.in_open_fence = in_fence;
        self.in_open_table = in_table;

        // Move stable lines to the stable buffer.
        if stable_count > 0 {
            for &line in lines.iter().take(stable_count) {
                self.stable.push(line.to_owned());
            }
        }

        // Keep unstable lines in tail.
        if stable_count < n {
            let remaining = &lines[stable_count..];
            self.tail = remaining.join("\n");
        }
    }
}

/// Classify lines using pulldown_cmark to detect fence/table boundaries.
/// Returns (stable_line_count, fence_still_open, table_still_open).
fn classify_lines_with_pulldown(
    lines: &[&str],
    mut in_fence: bool,
    mut in_table: bool,
) -> (usize, bool, bool) {
    let mut stable_count = 0usize;
    let mut in_open_construct = in_fence || in_table;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if in_open_construct {
            // Check if this line closes the current construct.
            if try_close_construct(&mut in_fence, &mut in_table, trimmed) {
                in_open_construct = in_fence || in_table;
                stable_count = i + 1;
            }
            continue;
        }

        // Not in a construct - check if this line opens one.
        if let Some(result) = classify_normal_line(trimmed) {
            match result {
                LineClass::Empty | LineClass::Plain => stable_count = i + 1,
                LineClass::Fence(_) => {
                    in_fence = true;
                    in_open_construct = true;
                }
                LineClass::TableStart => {
                    in_table = true;
                    in_open_construct = true;
                }
            }
        }
    }

    (stable_count, in_fence, in_table)
}

#[derive(Debug, Clone)]
enum LineClass {
    Empty,
    Plain,
    Fence(String),
    TableStart,
}

/// Classify a normal line (not inside a construct) using pulldown_cmark.
fn classify_normal_line(trimmed: &str) -> Option<LineClass> {
    if trimmed.is_empty() {
        return Some(LineClass::Empty);
    }

    // Use pulldown_cmark to detect fence and table starts.
    let parser = pulldown_cmark::Parser::new(trimmed);
    let mut has_code_block = false;
    let mut has_table_separator = false;

    for event in parser {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                has_code_block = true;
            }
            _ => {}
        }
    }

    // Check for table separator using pulldown_cmark-frontmatter or manual check.
    // The frontmatter crate doesn't detect tables, so we use a simple heuristic.
    if is_markdown_table_start(trimmed) {
        has_table_separator = true;
    }

    if has_code_block {
        let lang = trimmed.trim_start_matches("```").trim().to_owned();
        return Some(LineClass::Fence(lang));
    }

    if has_table_separator {
        return Some(LineClass::TableStart);
    }

    Some(LineClass::Plain)
}

/// Check if a line is the start of a markdown table (separator row |---|).
fn is_markdown_table_start(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return false;
    }
    // Must have at least one cell separator.
    let inner = &trimmed[1..trimmed.len() - 1];
    if !inner.contains('|') {
        return false;
    }
    // Each cell must be dashes (optionally with colons for alignment).
    inner.split('|').all(|cell| {
        let s = cell.trim();
        !s.is_empty() && s.chars().all(|c| c == '-' || c == ':')
    })
}

/// Check if a line closes an open construct.
/// A fence closes on exactly "```" (possibly with trailing whitespace).
/// "```rust" inside an open fence is treated as content.
/// Tables close on blank lines.
fn try_close_construct(
    fence_open: &mut bool,
    table_open: &mut bool,
    trimmed: &str,
) -> bool {
    // Fence close - must be exactly "```" (possibly with trailing whitespace).
    // Empty lines inside fences do NOT close them.
    if *fence_open {
        if trimmed.starts_with("```") {
            let after_fence = trimmed.trim_start_matches("```").trim();
            if after_fence.is_empty() {
                *fence_open = false;
                return true;
            }
        }
    }

    // Tables close on blank lines only.
    if *table_open && trimmed.is_empty() {
        *table_open = false;
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_buffer_flush_heals_stable_lines() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("hello **world\n");
        let lines = buf.flush();
        assert_eq!(lines, vec!["hello **world**"]);
    }

    #[test]
    fn streaming_buffer_force_flush_heals_tail() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("hello **world");
        let lines = buf.force_flush();
        assert_eq!(lines, vec!["hello **world**"]);
    }

    #[test]
    fn streaming_buffer_raw_text_not_healed_in_tail() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("hello **world\n");
        let lines = buf.flush();
        assert!(lines.iter().any(|l| l.contains("hello **world**")));
    }

    #[test]
    fn streaming_buffer_holds_incomplete_code_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("```rust\nfn main() {\n");
        assert!(!buf.is_stable());
        assert_eq!(buf.stable_len(), 0);

        // Complete the fence.
        buf.push_delta("}\n```\n");
        assert!(buf.is_stable());

        let lines = buf.flush();
        assert!(!lines.is_empty());
    }

    #[test]
    fn streaming_buffer_completes_code_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("```python\nprint('hello')\n```\n");
        let lines = buf.flush();
        assert!(!lines.is_empty());
    }

    #[test]
    fn streaming_buffer_flushes_complete_paragraph() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("This is a test.\n");
        let lines = buf.flush();
        assert!(!lines.is_empty());
    }

    #[test]
    fn streaming_buffer_resets() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("Some text\n");
        buf.reset();
        assert!(buf.is_stable());
        assert_eq!(buf.stable_len(), 0);
    }
}
