use std::collections::HashMap;

/// Cache for wrap_text results to avoid recomputing every frame.
/// Key is (text_hash, width) -> value is Vec<String> of wrapped lines.
#[derive(Clone)]
pub struct WrapCache {
    cache: HashMap<(u64, usize), Vec<String>>,
    access_order: Vec<(u64, usize)>,
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
        let text_hash = hash_text(text);
        let key = (text_hash, width);
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
        self.cache.insert(key, wrapped.clone());
        self.access_order.push(key);
        wrapped
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Check if a string contains a URL
fn contains_url(text: &str) -> bool {
    text.contains("http://") || text.contains("https://")
}

/// Fast hash using fxhash algorithm (simplified)
fn hash_text(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Wrap text into lines respecting word boundaries and preserving URLs
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    // Fast path: no URLs and fits width
    if !contains_url(text) && text.len() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if contains_url(word) {
            flush_and_add_url(word, width, &mut lines, &mut current);
        } else {
            add_normal_word(word, width, &mut lines, &mut current);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Flush current line and add URL as indivisible token
fn flush_and_add_url(word: &str, width: usize, lines: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        lines.push(current.clone());
        current.clear();
    }

    // URL as single token - kept intact even if > width
    if word.len() <= width {
        lines.push(word.to_string());
    } else {
        lines.push(word.to_string());
    }
}

/// Add normal word with word wrap
fn add_normal_word(word: &str, width: usize, lines: &mut Vec<String>, current: &mut String) {
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

/// Wrap text while preserving newlines from source.
pub fn wrap_text_preserving_newlines(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for line in text.split('\n') {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            result.push(String::new());
        } else if trimmed.len() <= width {
            result.push(trimmed.to_string());
        } else {
            result.extend(wrap_single_line(trimmed, width));
        }
    }

    result
}

/// Wrap a single line (no newlines) to width, preserving URLs
fn wrap_single_line(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if contains_url(word) {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            lines.push(word.to_string());
        } else if current.is_empty() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_preserved_intact() {
        let url = "https://example.com/some/very/long/path/that/might/exceed/width";
        let text = format!("Check this {}", url);
        let wrapped = wrap_text(&text, 40);

        for line in &wrapped {
            if line.contains("example.com") {
                assert!(line.contains("https://example.com"));
            }
        }
    }

    #[test]
    fn test_url_with_context() {
        let text = "Visit https://github.com for more info";
        let wrapped = wrap_text(text, 30);
        let has_url = wrapped.iter().any(|l| l.contains("github.com"));
        assert!(has_url, "URL should be in output: {:?}", wrapped);
    }

    #[test]
    fn test_normal_word_wrap() {
        let text = "This is a long piece of text that should be wrapped normally";
        let wrapped = wrap_text(text, 20);
        for line in &wrapped {
            assert!(line.len() <= 20, "Line too long: {}", line);
        }
    }

    #[test]
    fn test_empty_text() {
        let wrapped = wrap_text("", 40);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "");
    }

    #[test]
    fn test_zero_width() {
        let wrapped = wrap_text("hello", 0);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0], "");
    }

    #[test]
    fn test_url_only_line() {
        let url = "https://crates.io/crates/fxhash";
        let wrapped = wrap_text(url, 30);
        assert!(wrapped.iter().any(|l| l.contains("crates.io")));
    }
}
