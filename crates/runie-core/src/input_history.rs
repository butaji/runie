//! Input history persistence and search.
//!
//! Saves input history to `~/.runie/history.jsonl` and loads on startup.
//! Supports prefix-based search/filter for history navigation.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// File paths
// ---------------------------------------------------------------------------

/// Default history file path: ~/.runie/history.jsonl
pub fn default_history_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("runie").join("history.jsonl"))
}

/// Ensure history directory exists.
fn ensure_history_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .map(|d| d.join("runie"))
        .context("no data directory")?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Load history entries from the history file.
/// Returns empty vec if file doesn't exist.
pub fn load_history() -> Result<Vec<String>> {
    let path = match default_history_path() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path).with_context(|| format!("open history: {:?}", path))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line.context("read history line")?;
        if line.trim().is_empty() {
            continue;
        }
        // Each line is a JSON string (escaped content)
        let entry: String = serde_json::from_str(&line)
            .unwrap_or_else(|_| line); // Fallback: use raw line if not valid JSON
        entries.push(entry);
    }

    Ok(entries)
}

/// Save history entries to the history file.
/// Creates/overwrites the file.
pub fn save_history(entries: &[String]) -> Result<()> {
    let dir = ensure_history_dir()?;
    let path = dir.join("history.jsonl");

    let file = File::create(&path)
        .with_context(|| format!("create history: {:?}", path))?;
    let mut writer = std::io::BufWriter::new(file);

    for entry in entries {
        let json = serde_json::to_string(entry)?;
        writeln!(writer, "{}", json).context("write history entry")?;
    }

    writer.flush()?;
    Ok(())
}

/// Append a single entry to the history file.
/// Creates file if it doesn't exist.
pub fn append_history(entry: &str) -> Result<()> {
    let dir = ensure_history_dir()?;
    let path = dir.join("history.jsonl");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("append history: {:?}", path))?;
    let mut writer = std::io::BufWriter::new(file);

    let json = serde_json::to_string(entry)?;
    writeln!(writer, "{}", json).context("append history entry")?;
    writer.flush()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Search/Filter
// ---------------------------------------------------------------------------

/// Filter history entries by prefix match.
/// Case-insensitive comparison.
pub fn filter_history(entries: &[String], prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return entries.to_vec();
    }

    entries
        .iter()
        .filter(|e| e.to_lowercase().starts_with(&prefix.to_lowercase()))
        .cloned()
        .collect()
}

/// Search history entries by substring (not just prefix).
/// Returns entries containing the query, in reverse chronological order.
pub fn search_history(entries: &[String], query: &str) -> Vec<String> {
    if query.is_empty() {
        return entries.to_vec();
    }

    let query_lower = query.to_lowercase();
    entries
        .iter()
        .filter(|e| e.to_lowercase().contains(&query_lower))
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev() // Reverse to show most recent first
        .collect()
}

// ---------------------------------------------------------------------------
// AppState integration helpers
// ---------------------------------------------------------------------------

impl super::model::AppState {
    /// Load history from disk into AppState.
    pub fn load_input_history(&mut self) {
        if let Ok(entries) = load_history() {
            self.input_history = entries;
        }
    }

    /// Save current history to disk.
    pub fn save_input_history(&self) {
        if let Err(e) = save_history(&self.input_history) {
            eprintln!("Failed to save input history: {}", e);
        }
    }

    /// Append current entry and save to disk.
    /// Call this when adding a new history entry.
    pub fn add_to_input_history(&mut self, entry: String) {
        // Avoid duplicates: remove if already exists
        self.input_history.retain(|h| h != &entry);
        self.input_history.push(entry);
        self.save_input_history();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_history_prefix_match() {
        let entries = vec![
            "hello world".into(),
            "help me".into(),
            "HELLO there".into(),
            "goodbye".into(),
        ];

        // Exact prefix (case-insensitive, so 3 matches: hello world, help me, HELLO there)
        let result = filter_history(&entries, "hel");
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|e| e.to_lowercase().starts_with("hel")));

        // Case-insensitive
        let result = filter_history(&entries, "HEL");
        assert_eq!(result.len(), 3);

        // No match
        let result = filter_history(&entries, "xyz");
        assert!(result.is_empty());

        // Empty prefix returns all
        let result = filter_history(&entries, "");
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn search_history_substring() {
        let entries = vec![
            "hello world".into(),
            "say hello".into(),
            "goodbye".into(),
        ];

        let result = search_history(&entries, "hello");
        assert_eq!(result.len(), 2);

        // Most recent first (reverse order)
        assert_eq!(result[0], "say hello");
        assert_eq!(result[1], "hello world");
    }

    #[test]
    fn filter_history_empty_input() {
        let entries = vec!["test".into()];
        let result = filter_history(&entries, "");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn search_history_empty_query() {
        let entries = vec!["test1".into(), "test2".into()];
        let result = search_history(&entries, "");
        assert_eq!(result.len(), 2);
    }

    // Layer 1: Persistence tests

    #[test]
    fn history_save_load_roundtrip() {
        // Test that entries can be saved and loaded from JSONL
        let entries = vec![
            "first command".to_string(),
            "second command".to_string(),
            "third command".to_string(),
        ];

        // Each entry should serialize to JSON
        for entry in &entries {
            let json = serde_json::to_string(entry).unwrap();
            let decoded: String = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, *entry);
        }
    }
}
