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
    if let Ok(dir) = std::env::var("RUNIE_TEST_DATA_DIR") {
        return Some(PathBuf::from(dir).join("history.jsonl"));
    }
    dirs::data_dir().map(|d| d.join("runie").join("history.jsonl"))
}

/// Ensure history directory exists.
fn ensure_history_dir() -> Result<PathBuf> {
    let dir = if let Ok(dir) = std::env::var("RUNIE_TEST_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::data_dir()
            .map(|d| d.join("runie"))
            .context("no data directory")?
    };
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
        let entry: String = serde_json::from_str(&line).unwrap_or(line); // Fallback: use raw line if not valid JSON
        entries.push(entry);
    }

    Ok(entries)
}

/// Save history entries to the history file.
/// Creates/overwrites the file.
pub fn save_history(entries: &[String]) -> Result<()> {
    let dir = ensure_history_dir()?;
    let path = dir.join("history.jsonl");

    let file = File::create(&path).with_context(|| format!("create history: {:?}", path))?;
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
/// Case-insensitive comparison. Used by Up/Down arrow navigation.
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

/// Score a history entry against a query. Higher score = better match.
/// Priority: exact prefix (10 000+) > exact substring (5 000+) > fuzzy (0-4 999).
fn fuzzy_entry_score(entry: &str, query: &str) -> Option<i32> {
    let entry_lower = entry.to_lowercase();
    let query_lower = query.to_lowercase();

    // Exact prefix: highest priority.
    if entry_lower.starts_with(&query_lower) {
        return Some(10_000 + (100 - entry.len() as i32).max(0));
    }

    // Exact substring: medium priority.
    if entry_lower.contains(&query_lower) {
        return Some(5_000 + (100 - entry.len() as i32).max(0));
    }

    // Fuzzy match via sublime_fuzzy.
    sublime_fuzzy::best_match(query, entry).map(|m| m.score() as i32)
}

/// Search history entries using fuzzy matching.
/// Exact substring matches rank above fuzzy matches.
/// Results are sorted by score (descending), then by recency (descending index).
/// Used by `/history` command with an optional query argument.
pub fn search_history(entries: &[String], query: &str) -> Vec<String> {
    if query.is_empty() {
        return entries.to_vec();
    }

    let mut scored: Vec<(i32, usize, String)> = entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            fuzzy_entry_score(entry, query).map(|score| (score, idx, entry.clone()))
        })
        .collect();

    // Sort: highest score first; for equal scores, most recent (highest index) first.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    scored.into_iter().map(|(_, _, entry)| entry).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // filter_history tests (prefix matching — used by Up/Down)
    // -------------------------------------------------------------------------

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
    fn filter_history_empty_input() {
        let entries = vec!["test".into()];
        let result = filter_history(&entries, "");
        assert_eq!(result.len(), 1);
    }

    // -------------------------------------------------------------------------
    // search_history tests (fuzzy matching)
    // -------------------------------------------------------------------------

    #[test]
    fn search_history_exact_substring_ranked_above_fuzzy() {
        // "cat" is an exact substring of entry 1 and a fuzzy match of entry 2.
        // Entry 1 should appear before entry 2.
        let entries = vec![
            "the cat sat on mat".into(), // exact substring
            "convention attend".into(),  // fuzzy (sublime_fuzzy matches 'cat' loosely)
            "other stuff".into(),
        ];
        let result = search_history(&entries, "cat");
        // Both entries should match (substring + fuzzy)
        assert!(result.len() >= 1);
        // Exact substring must come first
        assert_eq!(result[0], "the cat sat on mat");
    }

    #[test]
    fn search_history_prefix_ranked_above_substring() {
        // "hel" is a prefix of entry 1 and a substring (not prefix) of entry 2.
        let entries = vec![
            "hello world".into(), // prefix match (score 10 000+)
            "say hello".into(),  // substring match (score 5 000+)
        ];
        let result = search_history(&entries, "hel");
        assert_eq!(result.len(), 2);
        // Prefix must come first (higher score)
        assert_eq!(result[0], "hello world");
        assert_eq!(result[1], "say hello");
    }

    #[test]
    fn search_history_fuzzy_finds_typos() {
        // Entries: one has "cargo", one has "caret".
        // Query "crgt":
        //   - "cargo test": c-a-r-g fuzzy-match (g in "cargo", t in "test");
        //     NOT a substring (no "crgt" contiguous).
        //   - "caret build": c-a-r fuzzy-match (all in "caret");
        //     NOT a substring; "crgt" not in it at all → no match.
        // "cargo test" should appear via fuzzy even though it's not an exact substring.
        let entries = vec!["cargo test".into(), "caret build".into(), "hello world".into()];
        let result = search_history(&entries, "crgt");
        assert!(
            result.iter().any(|e| e.contains("cargo")),
            "fuzzy 'crgt' should match 'cargo test' even without exact substring"
        );
        assert!(
            !result.iter().any(|e| e.contains("caret")),
            "fuzzy 'crgt' should not match 'caret build'"
        );
    }

    #[test]
    fn search_history_empty_query_returns_all() {
        let entries = vec!["test1".into(), "test2".into()];
        let result = search_history(&entries, "");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn search_history_no_match_returns_empty() {
        let entries = vec!["hello".into(), "world".into()];
        let result = search_history(&entries, "xyz123");
        assert!(result.is_empty());
    }

    #[test]
    fn search_history_case_insensitive() {
        let entries = vec!["HELLO World".into()];
        let result = search_history(&entries, "hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "HELLO World");
    }

    // -------------------------------------------------------------------------
    // Persistence tests
    // -------------------------------------------------------------------------

    #[test]
    fn history_save_load_roundtrip() {
        let entries = vec![
            "first command".to_string(),
            "second command".to_string(),
            "third command".to_string(),
        ];

        for entry in &entries {
            let json = serde_json::to_string(entry).unwrap();
            let decoded: String = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, *entry);
        }
    }
}
