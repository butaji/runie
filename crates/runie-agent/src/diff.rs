//! Unified diff generation for file edits

use similar::{Algorithm, ChangeTag, TextDiff};
use std::time::Duration;

const DIFF_DEADLINE_SECS: u64 = 5;

fn trim_line_end(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

/// Represents a line in a unified diff
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    /// Context line (unchanged)
    Context(String),
    /// Added line (starts with +)
    Added(String),
    /// Removed line (starts with -)
    Removed(String),
    /// File header
    Header(String),
    /// Hunk header (e.g., @@ -1,5 +1,7 @@)
    HunkHeader(String),
}

/// Represents a hunk in a unified diff
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Represents a complete unified diff
#[derive(Debug, Clone)]
pub struct UnifiedDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

/// Generates a unified diff between old and new content
pub fn generate_unified_diff(old_content: &str, new_content: &str) -> UnifiedDiff {
    if old_content == new_content {
        return UnifiedDiff {
            old_path: "a".to_string(),
            new_path: "b".to_string(),
            hunks: Vec::new(),
        };
    }

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .timeout(Duration::from_secs(DIFF_DEADLINE_SECS))
        .diff_lines(old_content, new_content);

    let mut builder = HunkBuilder::new();
    for change in diff.iter_all_changes() {
        builder.apply_change(change);
    }

    UnifiedDiff {
        old_path: "a".to_string(),
        new_path: "b".to_string(),
        hunks: builder.finish(),
    }
}

struct HunkBuilder {
    hunks: Vec<DiffHunk>,
    current: Vec<DiffLine>,
    old_start: usize,
    new_start: usize,
    old_len: usize,
    new_len: usize,
    old_line: usize,
    new_line: usize,
}

impl HunkBuilder {
    fn new() -> Self {
        Self {
            hunks: Vec::new(),
            current: Vec::new(),
            old_start: 1,
            new_start: 1,
            old_len: 0,
            new_len: 0,
            old_line: 1,
            new_line: 1,
        }
    }

    fn apply_change(&mut self, change: similar::Change<&str>) {
        match change.tag() {
            ChangeTag::Equal => self.apply_equal(),
            ChangeTag::Delete => self.apply_delete(change.value()),
            ChangeTag::Insert => self.apply_insert(change.value()),
        }
    }

    fn apply_equal(&mut self) {
        self.flush_current();
        self.old_line += 1;
        self.new_line += 1;
    }

    fn apply_delete(&mut self, value: &str) {
        self.start_hunk_if_needed();
        self.current
            .push(DiffLine::Removed(trim_line_end(value).to_string()));
        self.old_len += 1;
        self.old_line += 1;
    }

    fn apply_insert(&mut self, value: &str) {
        self.start_hunk_if_needed();
        self.current
            .push(DiffLine::Added(trim_line_end(value).to_string()));
        self.new_len += 1;
        self.new_line += 1;
    }

    fn start_hunk_if_needed(&mut self) {
        if self.current.is_empty() {
            self.old_start = self.old_line;
            self.new_start = self.new_line;
        }
    }

    fn flush_current(&mut self) {
        if self.current.is_empty() {
            return;
        }
        self.hunks.push(DiffHunk {
            header: format!(
                "@@ -{},{} +{},{} @@",
                self.old_start, self.old_len, self.new_start, self.new_len
            ),
            lines: std::mem::take(&mut self.current),
        });
        self.old_len = 0;
        self.new_len = 0;
    }

    fn finish(mut self) -> Vec<DiffHunk> {
        self.flush_current();
        self.hunks
    }
}

/// Generate a preview of an edit without applying it.
pub fn preview_edit(
    path: &std::path::Path,
    old: &str,
    new: &str,
) -> anyhow::Result<runie_core::EditPreview> {
    let original = std::fs::read_to_string(path)?;
    let proposed = original.replacen(old, new, 1);
    let diff = generate_unified_diff(&original, &proposed);
    let diff_str = render_diff_to_string(&diff, &path.to_string_lossy());
    Ok(runie_core::EditPreview::new(
        path.to_path_buf(),
        original,
        proposed,
        diff_str,
    ))
}

/// Renders a unified diff to string format
pub fn render_diff_to_string(diff: &UnifiedDiff, path: &str) -> String {
    let mut output = Vec::new();

    output.push(format!("--- {}", path));
    output.push(format!("+++ {}", path));

    for hunk in &diff.hunks {
        output.push(hunk.header.clone());
        for line in &hunk.lines {
            match line {
                DiffLine::Context(s) => output.push(format!(" {}", s)),
                DiffLine::Added(s) => output.push(format!("+{}", s)),
                DiffLine::Removed(s) => output.push(format!("-{}", s)),
                DiffLine::Header(s) => output.push(s.clone()),
                DiffLine::HunkHeader(s) => output.push(s.clone()),
            }
        }
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_empty_diff() {
        let content = "line1\nline2\nline3";
        let diff = generate_unified_diff(content, content);
        assert!(
            diff.hunks.is_empty()
                || diff
                    .hunks
                    .iter()
                    .all(|h| h.lines.iter().all(|l| matches!(l, DiffLine::Context(_))))
        );
    }

    #[test]
    fn single_line_addition() {
        let old = "line1\nline2";
        let new = "line1\nline2\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());

        let has_added = diff
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Added(s) if s == "line3"));
        assert!(has_added, "Should have added line3");
    }

    #[test]
    fn single_line_removal() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());

        let has_removed = diff
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Removed(s) if s == "line2"));
        assert!(has_removed, "Should have removed line2");
    }

    #[test]
    fn single_line_modification() {
        let old = "line1\nold_line\nline3";
        let new = "line1\nnew_line\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());

        let has_removed = diff
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Removed(s) if s == "old_line"));
        let has_added = diff
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Added(s) if s == "new_line"));

        assert!(has_removed, "Should have removed old_line");
        assert!(has_added, "Should have added new_line");
    }

    #[test]
    fn render_diff_to_string_format() {
        let old = "line1";
        let new = "line2";
        let diff = generate_unified_diff(old, new);
        let output = render_diff_to_string(&diff, "test.txt");

        assert!(output.contains("--- test.txt"));
        assert!(output.contains("+++ test.txt"));
        assert!(output.contains("-line1"));
        assert!(output.contains("+line2"));
    }

    #[test]
    fn empty_old_content() {
        let old = "";
        let new = "new content";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn empty_new_content() {
        let old = "old content";
        let new = "";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn multi_line_addition() {
        let old = "line1";
        let new = "line1\nline2\nline3";
        let diff = generate_unified_diff(old, new);
        let output = render_diff_to_string(&diff, "test.txt");

        assert!(output.contains("+line2"));
        assert!(output.contains("+line3"));
    }

    #[test]
    fn preview_generates_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let preview = preview_edit(&path, "world", "universe").unwrap();
        assert!(preview.diff.contains("---"), "diff should have file header");
        assert!(preview.diff.contains("+++"), "diff should have file header");
        assert!(
            preview.diff.contains("-hello world"),
            "diff should show removed line"
        );
        assert!(
            preview.diff.contains("+hello universe"),
            "diff should show added line"
        );
        assert_eq!(preview.original, "hello world");
        assert_eq!(preview.proposed, "hello universe");
    }

    #[test]
    fn preview_shows_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "line1\nline2\nline3").unwrap();

        let preview = preview_edit(&path, "line2", "modified").unwrap();
        assert!(
            preview.diff.contains("@@"),
            "diff should have hunk header with line numbers"
        );
    }

    #[test]
    fn diff_large_file_completes() {
        let old: String = (0..5_000).map(|i| format!("old line {}\n", i)).collect();
        let new: String = (0..5_000).map(|i| format!("new line {}\n", i)).collect();
        let diff = generate_unified_diff(&old, &new);
        // The function must return; hunks are expected for completely different content.
        assert!(!diff.hunks.is_empty());
    }
}
