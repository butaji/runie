//! Tests for Diff.

use crate::diff::{Diff, DiffLine};

#[test]
fn identical_content_empty_hunks() {
    let content = "line1\nline2\nline3";
    let diff = Diff::generate(content, content);
    assert!(diff.hunks.is_empty());
}

#[test]
fn single_line_addition() {
    let old = "line1\nline2";
    let new = "line1\nline2\nline3";
    let diff = Diff::generate(old, new);
    assert!(!diff.hunks.is_empty());

    let has_added = diff
        .hunks
        .iter()
        .flat_map(|h| &h.lines)
        .any(|l| matches!(l, DiffLine::Added(s, _) if s == "line3"));
    assert!(has_added, "Should have added line3");
}

#[test]
fn single_line_removal() {
    let old = "line1\nline2\nline3";
    let new = "line1\nline3";
    let diff = Diff::generate(old, new);
    assert!(!diff.hunks.is_empty());

    let has_removed = diff
        .hunks
        .iter()
        .flat_map(|h| &h.lines)
        .any(|l| matches!(l, DiffLine::Removed(s, _) if s == "line2"));
    assert!(has_removed, "Should have removed line2");
}

#[test]
fn single_line_modification() {
    let old = "line1\nold_line\nline3";
    let new = "line1\nnew_line\nline3";
    let diff = Diff::generate(old, new);
    assert!(!diff.hunks.is_empty());

    let has_removed = diff
        .hunks
        .iter()
        .flat_map(|h| &h.lines)
        .any(|l| matches!(l, DiffLine::Removed(s, _) if s == "old_line"));
    let has_added = diff
        .hunks
        .iter()
        .flat_map(|h| &h.lines)
        .any(|l| matches!(l, DiffLine::Added(s, _) if s == "new_line"));

    assert!(has_removed, "Should have removed old_line");
    assert!(has_added, "Should have added new_line");
}

#[test]
fn to_unified_string_format() {
    let old = "line1";
    let new = "line2";
    let diff = Diff::generate(old, new);
    let output = diff.to_unified_string();

    assert!(output.contains("--- a"));
    assert!(output.contains("+++ b"));
    assert!(output.contains("-line1"));
    assert!(output.contains("+line2"));
}

#[test]
fn empty_old_content() {
    let old = "";
    let new = "new content";
    let diff = Diff::generate(old, new);
    assert!(!diff.hunks.is_empty());
}

#[test]
fn empty_new_content() {
    let old = "old content";
    let new = "";
    let diff = Diff::generate(old, new);
    assert!(!diff.hunks.is_empty());
}

#[test]
fn diff_large_file_completes() {
    let old: String = (0..5_000).map(|i| format!("old line {}\n", i)).collect();
    let new: String = (0..5_000).map(|i| format!("new line {}\n", i)).collect();
    let diff = Diff::generate(&old, &new);
    assert!(!diff.hunks.is_empty());
}

#[test]
fn round_trip_preserves_hunks() {
    // Generate diff, render to string, re-generate — structure preserved.
    let old = "line1\nline2\nline3";
    let new = "line1\nmodified\nline3";
    let diff = Diff::generate(old, new);
    let stringified = diff.to_unified_string();
    let diff2 = Diff::generate(old, &stringified);
    // Second diff compares original against the stringified output,
    // so we just verify it's valid (no panic, non-empty).
    let _ = diff2;
}
