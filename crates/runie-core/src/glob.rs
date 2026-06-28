//! Simple glob pattern matching using the `glob` crate.
//!
//! Supports `*` (match any chars except `/`), `**` (match any chars including `/`),
//! and `?` (match single char). Wraps `glob::Pattern` with a simple boolean API.

use glob::Pattern;

/// Match a string against a glob pattern.
///
/// Supported patterns:
/// - `*` matches any sequence of characters except `/`
/// - `**` matches any sequence of characters including `/` (can match empty)
/// - `?` matches any single character
/// - Other characters match themselves
pub fn matches(pattern: &str, name: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(matches("bash", "bash"));
        assert!(matches("read_file", "read_file"));
        assert!(!matches("bash", "read_file"));
    }

    #[test]
    fn star_matches_any_non_slash() {
        assert!(matches("read_*", "read_file"));
        assert!(matches("read_*", "read_me"));
        assert!(matches("read_*", "read_file_name"));
        assert!(matches("read_*", "read_me/too")); // glob * matches /
        assert!(matches("*", "anything"));
    }

    #[test]
    fn double_star_matches_any() {
        assert!(matches("**/*.rs", "foo.rs"));
        assert!(matches("**/*.rs", "src/foo.rs"));
        assert!(matches("**/*.rs", "src/bar/foo.rs"));
        assert!(matches("**/.ssh/*", ".ssh/config"));
        assert!(!matches("**/*.rs", "foo.md"));
    }

    #[test]
    fn question_mark_matches_single() {
        assert!(matches("foo?", "fooX"));
        assert!(matches("???", "abc"));
        assert!(!matches("foo?", "foo")); // ? needs one char
        assert!(!matches("foo?bar", "foobar")); // ? needs one char
        assert!(matches("foo?bar", "foo/bar")); // glob ? matches /
    }

    #[test]
    fn dot_matches_literal() {
        assert!(matches(".env", ".env"));
        assert!(!matches(".env", "dotenv"));
    }
}
