use std::collections::HashMap;

/// Cache for wrap_text results to avoid recomputing every frame.
/// Key is (text, width) -> value is Vec<String> of wrapped lines.
#[derive(Clone)]
pub struct WrapCache {
    cache: HashMap<(String, usize), Vec<String>>,
    access_order: Vec<(String, usize)>,
    max_size: usize,
}

impl Default for WrapCache {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_size: 100,
        }
    }

    pub fn get_wrapped(&mut self, text: &str, width: usize) -> Vec<String> {
        let key = (text.to_string(), width);
        if let Some(cached) = self.cache.get(&key) {
            if let Some(pos) = self.access_order.iter().position(|k| *k == key) {
                self.access_order.remove(pos);
                self.access_order.push(key);
            }
            return cached.clone();
        }

        if self.cache.len() >= self.max_size {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.cache.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        let wrapped = wrap_text_preserving_newlines(text, width);
        self.cache.insert(key.clone(), wrapped.clone());
        self.access_order.push(key);
        wrapped
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Wrap text into lines respecting word boundaries
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Wrap text while preserving newlines from source.
pub fn wrap_text_preserving_newlines(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for line in text.split('\n') {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            result.push(String::new());
            continue;
        }

        if trimmed.len() <= width {
            result.push(trimmed.to_string());
        } else {
            result.extend(wrap_single_line(trimmed, width));
        }
    }

    result
}

/// Wrap a single line (no newlines) to width
fn wrap_single_line(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}
