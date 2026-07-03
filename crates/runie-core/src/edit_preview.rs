//! Edit preview for file mutations

use camino::Utf8PathBuf;

use serde::{Deserialize, Serialize};

use crate::diff::Diff;

/// Preview of a proposed file edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditPreview {
    /// Path to the file being edited.
    pub path: Utf8PathBuf,
    /// Original file content.
    pub original: String,
    /// Proposed file content.
    pub proposed: String,
    /// Canonical diff representation.
    pub diff: Diff,
}

impl EditPreview {
    /// Build an edit preview by generating a diff from original and proposed content.
    ///
    /// # Panics
    ///
    /// Panics if `path` is not valid UTF-8. All project paths should be valid UTF-8.
    pub fn new(path: Utf8PathBuf, original: String, proposed: String) -> Self {
        let diff = Diff::generate(&original, &proposed);
        Self {
            path,
            original,
            proposed,
            diff,
        }
    }

    /// Build from a `String` path (e.g. from event data).
    ///
    /// # Panics
    ///
    /// Panics if `path` is not valid UTF-8.
    #[cfg(test)]
    pub fn new_from_string(path: String, original: String, proposed: String) -> Self {
        Self::new(Utf8PathBuf::from(path), original, proposed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_preview_round_trip_through_json() {
        let preview = EditPreview::new(
            Utf8PathBuf::from("src/lib.rs"),
            "old content".into(),
            "new content".into(),
        );
        let json = serde_json::to_string(&preview).unwrap();
        let round_trip: EditPreview = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.path.as_str(), "src/lib.rs");
        assert_eq!(round_trip.original, "old content");
        assert_eq!(round_trip.proposed, "new content");
    }

    #[test]
    fn edit_preview_path_as_utf8_string() {
        let preview = EditPreview::new(Utf8PathBuf::from("src/lib.rs"), "a".into(), "b".into());
        // Utf8PathBuf is always valid UTF-8, no lossy conversion needed
        let path_str: &str = preview.path.as_str();
        assert_eq!(path_str, "src/lib.rs");
    }

    #[test]
    fn edit_preview_serialization_matches_pathbuf_format() {
        // Verify the JSON format is a plain string (same as PathBuf serialization)
        let preview = EditPreview::new(Utf8PathBuf::from("/foo/bar.rs"), "a".into(), "b".into());
        let json = serde_json::to_string(&preview).unwrap();
        // The path field should serialize as a plain string, not an object
        assert!(
            json.contains(r#""path":"/foo/bar.rs""#),
            "path should serialize as a plain JSON string, got: {json}"
        );
    }

    #[test]
    fn edit_preview_from_string_path() {
        let preview = EditPreview::new_from_string(
            "src/main.rs".into(),
            "original".into(),
            "modified".into(),
        );
        assert_eq!(preview.path.as_str(), "src/main.rs");
        assert_eq!(preview.original, "original");
        assert_eq!(preview.proposed, "modified");
    }
}
