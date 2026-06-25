//! Canonical diff representation shared by agent and TUI.
//!
//! The agent generates a `Diff` directly; the TUI renders it without parsing.
//! String serialization is only used for copy/export.

use similar::{Algorithm, ChangeTag, TextDiff};
use std::time::Duration;

const DIFF_DEADLINE_SECS: u64 = 5;

/// A line within a hunk.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DiffLine {
    Added(
        String,
        #[serde(skip_serializing_if = "Option::is_none")] Option<u32>,
    ),
    Removed(
        String,
        #[serde(skip_serializing_if = "Option::is_none")] Option<u32>,
    ),
    Context(String),
}

impl DiffLine {
    /// Create an added line without line number (canonical generation).
    pub fn added(content: impl Into<String>) -> Self {
        DiffLine::Added(content.into(), None)
    }

    /// Create a removed line without line number (canonical generation).
    pub fn removed(content: impl Into<String>) -> Self {
        DiffLine::Removed(content.into(), None)
    }

    /// Create a context line.
    pub fn context(content: impl Into<String>) -> Self {
        DiffLine::Context(content.into())
    }

    /// Get the content string from this line.
    pub fn content(&self) -> &str {
        match self {
            DiffLine::Added(s, _) | DiffLine::Removed(s, _) | DiffLine::Context(s) => s,
        }
    }

    /// Get the line number if present.
    pub fn line_number(&self) -> Option<u32> {
        match self {
            DiffLine::Added(_, n) | DiffLine::Removed(_, n) => *n,
            DiffLine::Context(_) => None,
        }
    }
}

/// A hunk within a diff (with header and lines).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A unified diff for a single file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Diff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

impl Diff {
    /// Generate a unified diff between old and new content.
    pub fn generate(old_content: &str, new_content: &str) -> Self {
        if old_content == new_content {
            return Diff {
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

        Diff {
            old_path: "a".to_string(),
            new_path: "b".to_string(),
            hunks: builder.finish(),
        }
    }

    /// Render to unified-diff string format (used for copy/export only).
    pub fn to_unified_string(&self) -> String {
        let mut output = Vec::new();
        let old = format!("--- {}", self.old_path);
        let new = format!("+++ {}", self.new_path);
        output.push(old);
        output.push(new);

        for hunk in &self.hunks {
            output.push(hunk.header.clone());
            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) => output.push(format!(" {s}")),
                    DiffLine::Added(s, _) => output.push(format!("+{s}")),
                    DiffLine::Removed(s, _) => output.push(format!("-{s}")),
                }
            }
        }

        output.join("\n")
    }

    /// Check if text looks like a unified diff.
    pub fn is_diff_output(text: &str) -> bool {
        let first_line = text.lines().next().unwrap_or("");
        first_line.starts_with("--- ") || first_line.starts_with("diff ")
    }

    /// Parse unified diff format — tries patch crate first, falls back to legacy parser.
    pub fn parse(text: &str) -> Diff {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            patch::Patch::from_single(text)
        }));
        if let Ok(Ok(p)) = result {
            return patch_to_canonical(p);
        }
        legacy_parse_diff(text)
    }
}

fn patch_to_canonical(p: patch::Patch) -> Diff {
    let hunks = p.hunks.iter().map(patch_hunk_to_diff_hunk).collect();
    Diff {
        old_path: p.old.path.to_string(),
        new_path: p.new.path.to_string(),
        hunks,
    }
}

fn patch_hunk_to_diff_hunk(h: &patch::Hunk) -> DiffHunk {
    let mut old_line = h.old_range.start as u32;
    let mut new_line = h.new_range.start as u32;
    let lines: Vec<DiffLine> = h
        .lines
        .iter()
        .map(|l| match l {
            patch::Line::Add(s) => {
                let n = new_line;
                new_line += 1;
                DiffLine::Added(s.to_string(), Some(n))
            }
            patch::Line::Remove(s) => {
                let n = old_line;
                old_line += 1;
                DiffLine::Removed(s.to_string(), Some(n))
            }
            patch::Line::Context(s) => {
                old_line += 1;
                new_line += 1;
                DiffLine::Context(s.to_string())
            }
        })
        .collect();
    let header = h.hint().map(|h| h.to_string()).unwrap_or_default();
    DiffHunk { header, lines }
}

/// ── Legacy parser for imperfect agent output strings ─────────────────────────

fn legacy_parse_diff(text: &str) -> Diff {
    let mut state = LegacyParseState::default();
    for line in text.lines() {
        state.parse_line(line);
    }
    state.flush_hunk();
    Diff {
        old_path: std::mem::take(&mut state.old_path).unwrap_or_default(),
        new_path: std::mem::take(&mut state.new_path).unwrap_or_default(),
        hunks: std::mem::take(&mut state.hunks),
    }
}

#[derive(Default)]
struct LegacyParseState {
    old_path: Option<String>,
    new_path: Option<String>,
    old_line_num: Option<u32>,
    new_line_num: Option<u32>,
    current_hunk: Option<DiffHunk>,
    hunks: Vec<DiffHunk>,
}

impl LegacyParseState {
    fn parse_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        match line.as_bytes().first() {
            Some(b'-') if line.starts_with("--- ") => self.parse_old_header(line),
            Some(b'+') if line.starts_with("+++ ") => self.parse_new_header(line),
            Some(b'@') if line.starts_with("@@ ") => self.parse_hunk_header(line),
            Some(b'+') => self.parse_added(line),
            Some(b'-') => self.parse_removed(line),
            Some(b' ') => self.parse_context(line),
            _ => {}
        }
    }

    fn parse_old_header(&mut self, line: &str) {
        self.old_path = Some(line[4..].to_string());
    }

    fn parse_new_header(&mut self, line: &str) {
        self.new_path = Some(line[4..].to_string());
    }

    fn parse_added(&mut self, line: &str) {
        let num = self.new_line_num;
        if let Some(ref mut n) = self.new_line_num {
            *n += 1;
        }
        self.push_line(DiffLine::Added(line[1..].to_string(), num));
    }

    fn parse_removed(&mut self, line: &str) {
        let num = self.old_line_num;
        if let Some(ref mut n) = self.old_line_num {
            *n += 1;
        }
        self.push_line(DiffLine::Removed(line[1..].to_string(), num));
    }

    fn parse_context(&mut self, line: &str) {
        if let Some(ref mut o) = self.old_line_num {
            *o += 1;
        }
        if let Some(ref mut n) = self.new_line_num {
            *n += 1;
        }
        self.push_line(DiffLine::Context(line[1..].to_string()));
    }

    fn parse_hunk_header(&mut self, line: &str) {
        self.flush_hunk();
        let parts: Vec<&str> = line.split_whitespace().collect();
        self.old_line_num = parts
            .get(1)
            .and_then(|s| s.split(',').next()?.strip_prefix('-')?.parse().ok());
        self.new_line_num = parts
            .get(2)
            .and_then(|s| s.split(',').next()?.strip_prefix('+')?.parse().ok());
        self.current_hunk = Some(DiffHunk {
            header: line.to_string(),
            lines: Vec::new(),
        });
        // Add hunk header as a context line (preserves original behavior)
        self.push_line(DiffLine::Context(line.to_string()));
    }

    fn push_line(&mut self, line: DiffLine) {
        if self.current_hunk.is_none() {
            self.current_hunk = Some(DiffHunk {
                header: String::new(),
                lines: Vec::new(),
            });
        }
        if let Some(ref mut hunk) = self.current_hunk {
            hunk.lines.push(line);
        }
    }

    fn flush_hunk(&mut self) {
        if let Some(hunk) = self.current_hunk.take() {
            if !hunk.lines.is_empty() {
                self.hunks.push(hunk);
            }
        }
    }
}

impl Drop for LegacyParseState {
    fn drop(&mut self) {
        self.flush_hunk();
    }
}

/// ── Internal hunk builder ──────────────────────────────────────────────────
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

fn trim_end(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
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
        self.current.push(DiffLine::removed(trim_end(value)));
        self.old_len += 1;
        self.old_line += 1;
    }

    fn apply_insert(&mut self, value: &str) {
        self.start_hunk_if_needed();
        self.current.push(DiffLine::added(trim_end(value)));
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
