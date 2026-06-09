//! Edit preview for file mutations

use std::path::PathBuf;

/// Preview of a proposed file edit.
#[derive(Clone, Debug, PartialEq)]
pub struct EditPreview {
    pub path: PathBuf,
    pub original: String,
    pub proposed: String,
    pub diff: String,
}

impl EditPreview {
    pub fn new(path: PathBuf, original: String, proposed: String, diff: String) -> Self {
        Self { path, original, proposed, diff }
    }
}
