//! Diff generation for file edits.
//!
//! Uses the canonical `runie_core::diff::Diff` type so the TUI can render
//! it directly without a string round-trip.

pub use runie_core::diff::Diff;
pub use runie_core::diff::DiffHunk;
pub use runie_core::diff::DiffLine;

/// Preview an edit without applying it.
pub fn preview_edit(
    path: &std::path::Path,
    old: &str,
    new: &str,
) -> anyhow::Result<runie_core::EditPreview> {
    let original = std::fs::read_to_string(path)?;
    let proposed = original.replacen(old, new, 1);
    // Canonical diff is generated inside EditPreview::new.
    Ok(runie_core::EditPreview::new(
        path.to_path_buf(),
        original,
        proposed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_preview_returns_canonical_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let preview = preview_edit(&path, "world", "universe").unwrap();
        assert_eq!(preview.original, "hello world");
        assert_eq!(preview.proposed, "hello universe");
        // Diff was generated canonically.
        assert!(!preview.diff.hunks.is_empty());
        let diff_str = preview.diff.to_unified_string();
        assert!(diff_str.contains("-hello world"), "diff should show removed line");
        assert!(diff_str.contains("+hello universe"), "diff should show added line");
    }
}
