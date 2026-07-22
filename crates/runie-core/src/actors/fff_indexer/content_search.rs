//! Content search helpers for FFF indexer.
//!
//! Provides functions for searching file contents with regex or literal matching.

use super::ContentMatch;

/// Perform a regex content search and return matches.
pub(super) fn find_regex_matches(
    content: &str,
    re: &regex::Regex,
    max_per_file: usize,
    path: &str,
) -> Vec<ContentMatch> {
    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max_per_file {
            break;
        }
        if let Some(m) = re.find(line) {
            let score = sublime_fuzzy::FuzzySearch::new(path, line)
                .best_match()
                .map(|r| r.score() as i32);
            matches.push(ContentMatch {
                path: path.to_owned(),
                line_number: (line_num + 1) as u64,
                col: m.start() + 1,
                line_content: line.to_owned(),
                fuzzy_score: score,
            });
        }
    }
    matches
}

/// Perform a literal content search and return matches.
pub(super) fn find_literal_matches(content: &str, query: &str, max_per_file: usize, path: &str) -> Vec<ContentMatch> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max_per_file {
            break;
        }
        if line.to_lowercase().contains(&query_lower) {
            let col = line.to_lowercase().find(&query_lower).unwrap_or(0) + 1;
            let score = sublime_fuzzy::FuzzySearch::new(path, line)
                .best_match()
                .map(|r| r.score() as i32);
            matches.push(ContentMatch {
                path: path.to_owned(),
                line_number: (line_num + 1) as u64,
                col,
                line_content: line.to_owned(),
                fuzzy_score: score,
            });
        }
    }
    matches
}
