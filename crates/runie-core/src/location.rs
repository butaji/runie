//! Location parsing for file:line:col patterns.
//!
//! Defines the [`Location`] enum and [`SearchQuery`] parser for the search index.
//! Replaces `fff_search::Location` with a local implementation.

use serde::{Deserialize, Serialize};

/// Location within a file (line, column, or range).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Location {
    /// Just the line number (e.g., `file:42`).
    Line(i32),
    /// Line and column (e.g., `file:42:5`).
    Position { line: i32, col: i32 },
    /// Line range (e.g., `file:10-20`).
    Range {
        start: (i32, i32),
        end: (i32, i32),
    },
}

/// Search query constraint — parsed from the query string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchConstraint {
    /// Glob pattern (e.g., `*.rs`).
    Glob(String),
    /// Negation filter (e.g., `!test/`).
    Not(String),
    /// Git status filter (e.g., `git:modified`).
    GitStatus(String),
}

/// A parsed search query with text component and constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SearchQuery {
    /// The fuzzy text component.
    pub text: String,
    /// Parsed constraints (glob, negation, git-status).
    pub constraints: Vec<SearchConstraint>,
    /// Location hint, if any.
    pub location: Option<Location>,
}

impl SearchQuery {
    /// Returns true if the query has any non-text components.
    pub fn has_constraints(&self) -> bool {
        !self.constraints.is_empty() || self.location.is_some()
    }

    /// Returns the raw text with constraints stripped.
    pub fn text_only(&self) -> &str {
        &self.text
    }

    /// Returns glob patterns from constraints.
    pub fn globs(&self) -> impl Iterator<Item = &str> {
        self.constraints.iter().filter_map(|c| match c {
            SearchConstraint::Glob(g) => Some(g.as_str()),
            _ => None,
        })
    }

    /// Returns negation patterns from constraints.
    pub fn negations(&self) -> impl Iterator<Item = &str> {
        self.constraints.iter().filter_map(|c| match c {
            SearchConstraint::Not(n) => Some(n.as_str()),
            _ => None,
        })
    }

    /// Returns git-status filter values.
    pub fn git_status_filters(&self) -> impl Iterator<Item = &str> {
        self.constraints.iter().filter_map(|c| match c {
            SearchConstraint::GitStatus(s) => Some(s.as_str()),
            _ => None,
        })
    }
}

/// Parse a search query string into a [`SearchQuery`].
///
/// Supports:
/// - Fuzzy text (e.g., `mylib`)
/// - Glob patterns (`*.rs`, `**/*.test.ts`)
/// - Negations (`!test/`, `!vendor/`)
/// - Git status (`git:modified`, `git:untracked`)
/// - Location hints (`file:42`, `file:42:5`, `file:10-20`)
pub fn parse_search_query(input: &str) -> SearchQuery {
    let input = input.trim();
    if input.is_empty() {
        return SearchQuery::default();
    }

    // First, extract the location suffix (`:line`, `:line:col`, `:line-col`).
    let (core, location) = extract_location(input);

    // Parse constraints and text from the core string.
    let mut text = String::new();
    let mut constraints = Vec::new();

    for token in core.split_whitespace() {
        if token.starts_with("*.") || token.starts_with("**/") || token.starts_with('*') {
            // Glob pattern
            constraints.push(SearchConstraint::Glob(token.to_owned()));
        } else if token.starts_with("git:") {
            constraints.push(SearchConstraint::GitStatus(token[4..].to_owned()));
        } else if token.starts_with('!') {
            constraints.push(SearchConstraint::Not(token[1..].to_owned()));
        } else if token.starts_with("git:") && token.len() > 4 {
            constraints.push(SearchConstraint::GitStatus(token[4..].to_owned()));
        } else if !text.is_empty() {
            text.push(' ');
            text.push_str(token);
        } else {
            text = token.to_owned();
        }
    }

    SearchQuery {
        text,
        constraints,
        location,
    }
}

/// Extract a location suffix from a query string.
fn extract_location(input: &str) -> (&str, Option<Location>) {
    // Look for `:line`, `:line:col`, `:start-end` at the end.
    // Try candidate colons from left to right; the first one that produces a valid
    // location suffix wins. This correctly handles cases like `lib.rs:42:5` where
    // the first `:` separates the filename from the location, and `src/lib.rs:10:5`
    // where the first `:` separates the path from the position.
    let bytes = input.as_bytes();

    // Collect all colon positions.
    let colon_positions: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter(|(_, &b)| b == b':')
        .map(|(i, _)| i)
        .filter(|&i| i > 0)
        .collect();

    // Try from leftmost colon to rightmost.
    for colon_idx in colon_positions {
        let before = &input[..colon_idx];
        let has_path_sep = before.contains('/') || before.contains('\\');
        let looks_like_filename = before.contains('.') && !before.ends_with('.');
        if !has_path_sep && !looks_like_filename {
            continue;
        }

        let location_part = &input[colon_idx + 1..];
        if let Some(loc) = parse_location_suffix(location_part) {
            let clean_before = before.trim_end_matches(|c| c == '/' || c == '\\');
            return (clean_before, Some(loc));
        }
    }

    (input, None)
}

/// Try to parse a location suffix (the part after the last `:`).
fn parse_location_suffix(part: &str) -> Option<Location> {
    // Try `line-col` or `line:col-col` range
    if part.contains('-') {
        return parse_range(part);
    }

    // Try `line:col` position
    if part.matches(':').count() == 1 {
        let (line_str, col_str) = part.split_once(':')?;
        let line = line_str.parse::<i32>().ok();
        let col = col_str.parse::<i32>().ok();
        if let Some(line) = line {
            return Some(Location::Position { line, col: col.unwrap_or(0) });
        }
    }

    // Try bare `line`
    part.parse::<i32>()
        .ok()
        .map(Location::Line)
}

/// Parse a range like `10-20` or `10:5-20:3`.
fn parse_range(part: &str) -> Option<Location> {
    let (start_str, end_str) = part.split_once('-')?;

    let start_has_colon = start_str.contains(':');
    let start_linecol = if start_has_colon {
        let (l, c) = start_str.split_once(':')?;
        let line = l.parse::<i32>().ok()?;
        let col = c.parse::<i32>().ok()?;
        (line, col)
    } else {
        let line = start_str.parse::<i32>().ok()?;
        (line, 0)
    };

    let end_has_colon = end_str.contains(':');
    let end_linecol = if end_has_colon {
        let (l, c) = end_str.split_once(':')?;
        let line = l.parse::<i32>().ok()?;
        let col = c.parse::<i32>().ok()?;
        (line, col)
    } else {
        // No colon: if start had no colon → line range (end_line=end, end_col=0).
        // If start had a colon → column range on same line (end_col=end, end_line=start_line).
        let n = end_str.parse::<i32>().ok()?;
        if start_has_colon {
            (start_linecol.0, n)
        } else {
            (n, 0)
        }
    };

    Some(Location::Range {
        start: start_linecol,
        end: end_linecol,
    })
}

/// Parse a `file:line:col` pattern into a path and optional location.
pub fn parse_location(query: &str) -> (&str, Option<Location>) {
    let (path, location) = extract_location(query);
    (path, location)
}

/// Returns the line number from a location, if present.
pub fn location_line(loc: &Location) -> i32 {
    match loc {
        Location::Line(l) => *l,
        Location::Position { line, .. } => *line,
        Location::Range { start, .. } => start.0,
    }
}

/// Returns the column number from a location, if present.
pub fn location_col(loc: &Location) -> Option<i32> {
    match loc {
        Location::Line(_) => None,
        Location::Position { col, .. } => Some(*col),
        Location::Range { start, .. } => Some(start.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_extracts_line_and_column() {
        let (path, loc) = parse_location("src/lib.rs:10:5");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(loc, Some(Location::Position { line: 10, col: 5 })));
    }

    #[test]
    fn parser_handles_missing_column() {
        let (path, loc) = parse_location("src/lib.rs:10");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(loc, Some(Location::Line(10))));
    }

    #[test]
    fn parser_handles_no_location() {
        let (path, loc) = parse_location("src/lib.rs");
        assert_eq!(path, "src/lib.rs");
        assert!(loc.is_none());
    }

    #[test]
    fn parser_handles_line_range() {
        let (path, loc) = parse_location("src/lib.rs:10-20");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(
            loc,
            Some(Location::Range { start, end })
            if start == (10, 0) && end == (20, 0)
        ));
    }

    #[test]
    fn parser_handles_column_range() {
        let (path, loc) = parse_location("src/lib.rs:10:5-20");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(
            loc,
            Some(Location::Range { start, end })
            if start == (10, 5) && end == (10, 20)
        ));
    }

    #[test]
    fn location_line_extraction() {
        let (_, loc) = parse_location("src/lib.rs:42:7");
        let loc = loc.unwrap();
        assert_eq!(location_line(&loc), 42);
        assert_eq!(location_col(&loc), Some(7));
    }

    #[test]
    fn location_line_only() {
        let (_, loc) = parse_location("src/lib.rs:99");
        let loc = loc.unwrap();
        assert_eq!(location_line(&loc), 99);
        assert_eq!(location_col(&loc), None);
    }

    // SearchQuery tests
    #[test]
    fn search_query_parse_fuzzy() {
        let q = parse_search_query("mylib");
        assert_eq!(q.text, "mylib");
        assert!(q.constraints.is_empty());
        assert!(q.location.is_none());
    }

    #[test]
    fn search_query_parse_glob() {
        let q = parse_search_query("*.rs");
        assert!(q.globs().eq(["*.rs"]));
    }

    #[test]
    fn search_query_parse_negation() {
        let q = parse_search_query("config !test/ !vendor/");
        assert_eq!(q.text, "config");
        assert!(q.negations().eq(["test/", "vendor/"]));
    }

    #[test]
    fn search_query_parse_git_status() {
        let q = parse_search_query("git:modified");
        assert!(q.git_status_filters().eq(["modified"]));
    }

    #[test]
    fn search_query_parse_location_hint() {
        let q = parse_search_query("lib.rs:42");
        assert_eq!(q.text, "lib.rs");
        assert!(matches!(q.location, Some(Location::Line(42))));
    }

    #[test]
    fn search_query_parse_location_with_column() {
        let q = parse_search_query("lib.rs:42:5");
        assert_eq!(q.text, "lib.rs");
        assert!(matches!(q.location, Some(Location::Position { line: 42, col: 5 })));
    }

    #[test]
    fn search_query_parse_mixed() {
        let q = parse_search_query("config yaml !test/ git:modified *.rs");
        assert_eq!(q.text, "config yaml");
        assert!(q.globs().eq(["*.rs"]));
        assert!(q.negations().eq(["test/"]));
        assert!(q.git_status_filters().eq(["modified"]));
    }
}
