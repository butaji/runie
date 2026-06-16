//! Edit preview for file mutations

use std::path::PathBuf;

use crate::diff::Diff;

/// Preview of a proposed file edit.
#[derive(Clone, Debug, PartialEq)]
pub struct EditPreview {
    /// Path to the file being edited.
    pub path: PathBuf,
    /// Original file content.
    pub original: String,
    /// Proposed file content.
    pub proposed: String,
    /// Canonical diff representation.
    pub diff: Diff,
}

impl EditPreview {
    /// Build an edit preview by generating a diff from original and proposed content.
    pub fn new(path: PathBuf, original: String, proposed: String) -> Self {
        let diff = Diff::generate(&original, &proposed);
        Self {
            path,
            original,
            proposed,
            diff,
        }
    }
}

