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

    /// Returns true if there's incomplete content being streamed (tail is non-empty).
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
