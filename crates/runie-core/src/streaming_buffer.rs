use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 50;

fn close_remaining(openers: Vec<char>, result: &mut String) {
    for opener in openers.into_iter().rev() {
        match opener {
            '`' => result.push('`'),
            '*' => result.push_str("**"),
            '_' => result.push('_'),
            '~' => result.push_str("~~"),
            '[' => result.push_str("]()"),
            _ => {}
        }
    }
}

fn is_triple_backtick(chars: &std::iter::Peekable<std::str::Chars>) -> bool {
    chars.clone().nth(1) == Some('`')
}

fn handle_double(
    c: char,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
    openers: &mut Vec<char>,
) {
    if chars.peek() == Some(&c) {
        chars.next();
        result.push(c);
        if openers.last() == Some(&c) {
            openers.pop();
            result.push(c);
        } else {
            openers.push(c);
        }
    }
}

fn handle_single(c: char, openers: &mut Vec<char>, blocker: Option<char>) {
    if openers.last() == Some(&c) {
        openers.pop();
    } else if blocker.map_or(true, |b| openers.last() != Some(&b)) {
        openers.push(c);
    }
}

fn process_markdown_char(
    c: char,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
    openers: &mut Vec<char>,
) {
    match c {
        '*' => handle_double('*', chars, result, openers),
        '`' => {
            if !is_triple_backtick(chars) {
                handle_single('`', openers, None);
            }
        }
        '_' => handle_single('_', openers, Some('*')),
        '~' => handle_double('~', chars, result, openers),
        '[' => openers.push('['),
        ']' => {
            if openers.last() == Some(&'[') {
                openers.pop();
            }
        }
        _ => {}
    }
}

/// Heals incomplete inline markdown spans in text for display purposes only.
/// Does NOT modify valid/closed syntax or plain text.
/// Examples:
///   "hello **world"  → "hello **world**"
///   "use `rust"      → "use `rust`"
///   "see [docs"      → "see [docs]()"
pub fn heal_markdown(text: &str) -> String {
    let mut openers: Vec<char> = Vec::new();
    let mut result = String::with_capacity(text.len() + 32);
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        result.push(c);
        process_markdown_char(c, &mut chars, &mut result, &mut openers);
    }

    close_remaining(openers, &mut result);
    result
}

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
        self.stable.drain(..).filter(|l| !l.is_empty()).map(|l| heal_markdown(&l)).collect()
    }

    pub fn force_flush(&mut self) -> Vec<String> {
        self.last_flush = Some(Instant::now());
        let mut lines: Vec<String> = self.stable.drain(..).map(|l| heal_markdown(&l)).collect();
        if !self.tail.is_empty() {
            lines.push(heal_markdown(&self.tail));
            self.tail.clear();
        }
        lines
    }

    pub fn tail(&self) -> &str {
        &self.tail
    }

    pub fn is_stable(&self) -> bool {
        self.tail.is_empty() && self.open_fence.is_none() && !self.open_table
    }

    pub fn has_pending_content(&self) -> bool {
        !self.tail.is_empty() || self.open_fence.is_some() || self.open_table
    }

    #[cfg(test)]
    pub fn stable_len(&self) -> usize {
        self.stable.len()
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

        let (stable_count, fence_open, table_open) =
            classify_lines(&lines, self.open_fence.clone(), self.open_table);

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

fn classify_lines(
    lines: &[&str],
    mut fence_open: Option<String>,
    mut table_open: bool,
) -> (usize, Option<String>, bool) {
    let mut stable_count = 0usize;
    let mut in_open_construct = fence_open.is_some() || table_open;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if in_open_construct {
            if try_close_construct(&mut fence_open, &mut table_open, trimmed) {
                in_open_construct = fence_open.is_some() || table_open;
                stable_count = i + 1;
            }
            continue;
        }

        if let Some(result) = classify_normal_line(trimmed) {
            match result {
                LineClass::Empty | LineClass::Plain => stable_count = i + 1,
                LineClass::Fence(lang) => {
                    fence_open = Some(lang);
                    in_open_construct = true;
                }
                LineClass::TableStart => {
                    table_open = true;
                    in_open_construct = true;
                }
            }
        }
    }

    (stable_count, fence_open, table_open)
}

#[derive(Debug, Clone)]
enum LineClass {
    Empty,
    Plain,
    Fence(String),
    TableStart,
}

fn classify_normal_line(trimmed: &str) -> Option<LineClass> {
    if trimmed.is_empty() {
        return Some(LineClass::Empty);
    }
    if trimmed.starts_with("```") {
        let lang = trimmed.trim_start_matches("```").trim().to_string();
        return Some(LineClass::Fence(lang));
    }
    if is_table_separator(trimmed) {
        return Some(LineClass::TableStart);
    }
    Some(LineClass::Plain)
}

fn try_close_construct(
    fence_open: &mut Option<String>,
    table_open: &mut bool,
    trimmed: &str,
) -> bool {
    if fence_open.is_some() && trimmed.starts_with("```") {
        *fence_open = None;
        return true;
    }
    if *table_open && trimmed.is_empty() {
        *table_open = false;
        return true;
    }
    false
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
    fn heal_markdown_closes_unclosed_bold() {
        assert_eq!(heal_markdown("hello **world"), "hello **world**");
        assert_eq!(heal_markdown("**bold start"), "**bold start**");
    }

    #[test]
    fn heal_markdown_closes_unclosed_italic() {
        assert_eq!(heal_markdown("hello _world"), "hello _world_");
        assert_eq!(heal_markdown("italic _start"), "italic _start_");
    }

    #[test]
    fn heal_markdown_closes_unclosed_inline_code() {
        assert_eq!(heal_markdown("use `rust"), "use `rust`");
        assert_eq!(heal_markdown("code `snippet"), "code `snippet`");
    }

    #[test]
    fn heal_markdown_closes_unclosed_link() {
        assert_eq!(heal_markdown("see [docs"), "see [docs]()");
        assert_eq!(heal_markdown("[link"), "[link]()");
    }

    #[test]
    fn heal_markdown_leaves_closed_syntax_unchanged() {
        assert_eq!(heal_markdown("hello **world** and `code`"), "hello **world** and `code`");
        assert_eq!(heal_markdown("**bold** and _italic_ and `code`"), "**bold** and _italic_ and `code`");
    }

    #[test]
    fn heal_markdown_leaves_plain_text_unchanged() {
        assert_eq!(heal_markdown("just plain text"), "just plain text");
        assert_eq!(heal_markdown(""), "");
    }

    #[test]
    fn heal_markdown_handles_multiple_unclosed_spans() {
        assert_eq!(heal_markdown("**bold and `code"), "**bold and `code`**");
    }

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
        buf.push_delta("hello **world\nmore **stuff");
        let lines = buf.flush();
        assert_eq!(lines, vec!["hello **world**"]);
        assert_eq!(buf.tail(), "more **stuff");
    }
}
