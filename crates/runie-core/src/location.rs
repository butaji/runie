//! Location parsing for file:line:col patterns.
//!
//! Defines the [`Location`] enum and [`SearchQuery`] parser for the search index.
//! Replaces `fff_search::Location` with a local implementation.

use std::sync::LazyLock;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Compiled regex for location suffix extraction.
///
/// Matches the following patterns at the end of a string:
/// - `N`           → `Location::Line(N)`
/// - `N:M`         → `Location::Position { line: N, col: M }`
/// - `N-M`         → `Location::Range { start: (N, 0), end: (M, 0) }`
/// - `N:M-P`       → `Location::Range { start: (N, M), end: (N, P) }` (column range)
/// - `N:M-P:Q`     → `Location::Range { start: (N, M), end: (P, Q) }` (full range)
///
/// The regex uses greedy matching to correctly handle paths containing colons.
static LOCATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?<path>[^:]+):(?<a>\d+)(?::(?<b>\d+))?(?:-(?:(?<c>\d+)(?::(?<d>\d+))?))?$",
    )
    .expect("location regex is valid")
});

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
        } else if let Some(stripped) = token.strip_prefix("git:") {
            constraints.push(SearchConstraint::GitStatus(stripped.to_owned()));
        } else if let Some(stripped) = token.strip_prefix('!') {
            constraints.push(SearchConstraint::Not(stripped.to_owned()));
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

/// Extract a location suffix from a query string using a compiled regex.
///
/// Tries to match a location pattern at the end of the string. For the prefix
/// to be treated as a valid path (and not fuzzy text), it must contain a path
/// separator or look like a filename (contain a dot).
fn extract_location(input: &str) -> (&str, Option<Location>) {
    // Try to match the compiled location regex at the end of the string.
    if let Some(caps) = LOCATION_RE.captures(input) {
        let path = caps.name("path").map(|m| m.as_str()).unwrap_or("");

        // Only accept as a path if it looks like a filename or contains a path separator.
        let looks_like_path =
            path.contains('/') || path.contains('\\') || (path.contains('.') && !path.ends_with('.'));
        if !looks_like_path {
            return (input, None);
        }

        let a = caps["a"].parse::<i32>().ok();
        let b = caps.name("b").and_then(|m| m.as_str().parse::<i32>().ok());
        let c = caps.name("c").and_then(|m| m.as_str().parse::<i32>().ok());
        let d = caps.name("d").and_then(|m| m.as_str().parse::<i32>().ok());

        let loc = match (a, b, c, d) {
            // `N` → Line
            (Some(line), None, None, None) => Some(Location::Line(line)),
            // `N:M` → Position
            (Some(line), Some(col), None, None) => Some(Location::Position { line, col }),
            // `N:M-P` → Range (column range: start line/col, same line, end col)
            (Some(line), Some(col_s), Some(end_col), None) => {
                Some(Location::Range { start: (line, col_s), end: (line, end_col) })
            }
            // `N:M-P:Q` → Range (full range)
            (Some(line), Some(col_s), Some(end_line), Some(end_col)) => {
                Some(Location::Range { start: (line, col_s), end: (end_line, end_col) })
            }
            // `N-P` → Range (line range: end_col=0)
            (Some(start_line), None, Some(end_line), None) => {
                Some(Location::Range { start: (start_line, 0), end: (end_line, 0) })
            }
            _ => None,
        };

        if let Some(loc) = loc {
            let clean_path = path.trim_end_matches(['/', '\\']);
            return (clean_path, Some(loc));
        }
    }

    (input, None)
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
