//! Grok Build SSE fixture strings — shared by runie-agent and runie-provider tests.
//!
//! Uses `include_dir!` to scan the fixture directory at compile time.
//!
//! ## Deterministic Sanitization
//!
//! Captured Grok output contains non-deterministic elements like timestamps, IDs,
//! and temp paths. The sanitizer replaces these with stable placeholders.

use std::collections::HashMap;
use std::sync::LazyLock;

use include_dir::Dir;
use regex::Regex;

/// The fixture directory.
static FIXTURE_DIR: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/fixtures/grok_build");

/// Lazy-loaded fixture contents (raw, before sanitization).
static FIXTURES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    FIXTURE_DIR
        .files()
        .map(|file| {
            let name = file.path().file_name().unwrap().to_str().unwrap();
            let contents = file.contents_utf8().unwrap_or_default();
            (name, contents)
        })
        .collect()
});

// ── Sanitization patterns ────────────────────────────────────────────────────

/// Timestamp pattern: ISO 8601 timestamps
static TIMESTAMP_PAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z").unwrap());

/// UUID pattern
static UUID_PAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap());

/// Session ID pattern
static SESSION_PAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"sess_[a-zA-Z0-9]+").unwrap());

/// Sanitize a fixture string, replacing non-deterministic elements.
fn sanitize(text: &str) -> String {
    let text = TIMESTAMP_PAT
        .replace_all(text, "1970-01-01T00:00:00Z")
        .to_string();
    let text = UUID_PAT
        .replace_all(&text, "00000000-0000-0000-0000-000000000000")
        .to_string();
    let text = SESSION_PAT.replace_all(&text, "sess_fixture").to_string();
    text
}

/// All Grok Build fixture names.
pub const ALL_FIXTURES: &[&str] = &["sample.sse"];

/// Load a raw (unsanitized) fixture by name. Panics if the name is unknown.
pub fn raw_fixture(name: &str) -> String {
    FIXTURES
        .get(name)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| panic!("unknown grok_build fixture: {name}"))
}

/// Load a sanitized fixture by name. Non-deterministic elements are replaced
/// with stable placeholders for deterministic test comparisons.
pub fn sanitized_fixture(name: &str) -> String {
    sanitize(&raw_fixture(name))
}

/// Check if a fixture exists.
pub fn has_fixture(name: &str) -> bool {
    FIXTURES.contains_key(name)
}

/// List all available fixture names.
pub fn fixture_names() -> Vec<&'static str> {
    FIXTURES.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_sanitization() {
        let input = "2024-06-15T12:30:45Z";
        let sanitized = sanitize(input);
        assert!(sanitized.contains("1970-01-01T00:00:00Z"));
        assert!(!sanitized.contains("2024-06-15T12:30:45Z"));
    }

    #[test]
    fn uuid_sanitization() {
        let input = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let sanitized = sanitize(input);
        assert!(sanitized.contains("00000000-0000-0000-0000-000000000000"));
        assert!(!sanitized.contains("a1b2c3d4"));
    }

    #[test]
    fn session_id_sanitization() {
        let input = "sess_abc123XYZ";
        let sanitized = sanitize(input);
        assert!(sanitized.contains("sess_fixture"));
        assert!(!sanitized.contains("abc123XYZ"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(sanitize(""), "");
    }

    #[test]
    fn no_changes_needed() {
        let input = "model: grok-3, content: Hello";
        let sanitized = sanitize(input);
        assert_eq!(sanitized, input);
    }
}
