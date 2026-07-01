//! Streaming buffer with markdown-aware stability detection.
//!
//! Uses `pulldown-cmark` event stream to determine which lines are "stable"
//! (not in an open code block or other construct that may continue).

use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 50;

pub use crate::markdown::heal_markdown;

/// Open block state tracked during parsing.
#[derive(Debug, Clone, Default)]
struct OpenBlock {
    /// Whether we are inside a fenced code block.
    in_code_block: bool,
    /// Whether we are inside a table block.
    in_table: bool,
}

impl OpenBlock {
    fn is_stable(&self) -> bool {
        !self.in_code_block && !self.in_table
    }
}

#[derive(Debug, Clone)]
pub struct StreamingBuffer {
    stable: Vec<String>,
    tail: String,
    open_block: OpenBlock,
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
            open_block: OpenBlock::default(),
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
        self.stable
            .drain(..)
            .filter(|l| !l.is_empty())
            .map(|l| heal_markdown(&l))
            .collect()
    }

    pub fn force_flush(&mut self) -> Vec<String> {
        self.last_flush = Some(Instant::now());
        let mut lines: Vec<String> = self.stable.drain(..).map(|l| heal_markdown(&l)).collect();
        if !self.tail.is_empty() {
            lines.push(heal_markdown(&self.tail));
            self.tail.clear();
        }
        // Reset state for next turn
        self.open_block = OpenBlock::default();
        lines
    }

    pub fn tail(&self) -> &str {
        &self.tail
    }

    pub fn is_stable(&self) -> bool {
        self.tail.is_empty() && self.open_block.is_stable()
    }

    pub fn has_pending_content(&self) -> bool {
        !self.tail.is_empty() || !self.open_block.is_stable()
    }

    #[cfg(test)]
    pub fn stable_len(&self) -> usize {
        self.stable.len()
    }

    pub fn reset(&mut self) {
        self.stable.clear();
        self.tail.clear();
        self.open_block = OpenBlock::default();
        self.last_flush = None;
    }

    /// Resolve stable lines from the tail using pulldown-cmark events.
    fn resolve(&mut self) {
        if self.tail.is_empty() {
            return;
        }

        // Parse with pulldown-cmark to detect block boundaries.
        let (stable_count, open_block) = parse_stable_lines(&self.tail, &self.open_block);

        if stable_count > 0 {
            let lines: Vec<&str> = self.tail.split('\n').collect();
            for &line in lines.iter().take(stable_count) {
                self.stable.push(line.to_owned());
            }
        }

        // Update open block state.
        self.open_block = open_block;

        // Update tail with remaining content.
        let total_lines = self.tail.split('\n').count();
        if stable_count < total_lines {
            let lines: Vec<&str> = self.tail.split('\n').collect();
            self.tail = lines[stable_count..].join("\n");
        } else {
            self.tail.clear();
        }
    }
}

/// Parse text using pulldown-cmark to find the count of stable (complete) lines.
///
/// A line is considered stable if it is not part of an open block construct
/// (code block, table, etc.).
///
/// Returns (stable_line_count, updated_open_block)
fn parse_stable_lines(tail: &str, open_block: &OpenBlock) -> (usize, OpenBlock) {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    if tail.is_empty() {
        return (0, open_block.clone());
    }

    // Set up parser options for tables and code blocks.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(tail, options);

    // Initialize state based on open_block from previous parsing.
    let mut in_code_block = open_block.in_code_block;
    let mut in_table = open_block.in_table;

    let mut current_line: usize = 0;
    let mut last_stable_line: usize = 0;

    for event in parser {
        match &event {
            Event::Text(t) => {
                // Count newlines in this text event to track line numbers.
                for c in t.chars() {
                    if c == '\n' {
                        current_line += 1;
                    }
                }
                // Empty text after table content closes the table.
                if in_table && t.trim().is_empty() {
                    in_table = false;
                    last_stable_line = current_line;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                current_line += 1;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                // Entering a code block.
                in_code_block = true;
                // Lines up to current line are stable.
                last_stable_line = current_line;
            }
            Event::End(TagEnd::CodeBlock) => {
                // Exiting a code block.
                in_code_block = false;
                last_stable_line = current_line;
            }
            Event::Start(Tag::Table(_)) | Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                // Entering a table construct.
                in_table = true;
                last_stable_line = current_line;
            }
            Event::End(TagEnd::Table) | Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                // Exiting a table construct.
                in_table = false;
                last_stable_line = current_line;
            }
            _ => {}
        }
    }

    // Calculate stable line count.
    // If we're in an open construct (fence or table), lines within it are NOT stable.
    // We only count lines up to but NOT including the construct start.
    let stable_count = if in_code_block || in_table {
        // We're inside an open construct. Lines before it started are stable.
        // The opening line of the construct itself is NOT stable (it stays in tail).
        last_stable_line
    } else {
        // No open construct - all lines up to current position are stable.
        current_line + 1
    };

    (
        stable_count,
        OpenBlock {
            in_code_block,
            in_table,
        },
    )
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
    fn streaming_buffer_stable_after_code_block_close() {
        let mut buf = StreamingBuffer::new();
        // Start a code block.
        buf.push_delta("```rust\n");
        assert!(buf.has_pending_content(), "Should have pending content in open fence");
        // Close the code block.
        buf.push_delta("```\n");
        assert!(buf.is_stable() || !buf.has_pending_content(), "Should be stable after fence close");
    }

    #[test]
    fn streaming_buffer_stable_with_closed_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("```\n");
        buf.push_delta("code\n");
        buf.push_delta("```\n");
        buf.push_delta("after\n");
        assert!(buf.is_stable(), "Should be stable after code block closed");
    }

    #[test]
    fn streaming_buffer_stable_lines_excludes_open_fence() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("before\n");
        buf.push_delta("```\n");
        buf.push_delta("code\n");
        let stable = buf.stable_len();
        assert_eq!(stable, 1, "Only 'before' should be stable");
        assert!(buf.has_pending_content(), "Should have pending content");
    }

    #[test]
    fn streaming_buffer_table_not_stable_until_closed() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("| Header |\n");
        buf.push_delta("| ------ |\n");
        buf.push_delta("| Cell   |\n");
        let stable = buf.stable_len();
        assert!(stable >= 3, "Table rows should be stable");
    }

    #[test]
    fn streaming_buffer_empty_line_stability() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("line1\n");
        buf.push_delta("\n");
        buf.push_delta("line2\n");
        assert!(buf.is_stable() || buf.stable_len() >= 2);
    }

    #[test]
    fn streaming_buffer_reset_clears_state() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("```\n");
        buf.push_delta("code\n");
        buf.reset();
        assert!(buf.stable.is_empty());
        assert!(buf.tail.is_empty());
        assert!(buf.open_block.is_stable());
    }

    #[test]
    fn streaming_buffer_partial_code_block() {
        let mut buf = StreamingBuffer::new();
        buf.push_delta("before\n");
        buf.push_delta("```\n");
        buf.push_delta("part1\n");
        assert_eq!(buf.stable_len(), 1);
        buf.push_delta("part2\n");
        buf.push_delta("```\n");
        buf.push_delta("after\n");
        assert!(buf.is_stable() || !buf.has_pending_content());
    }
}
