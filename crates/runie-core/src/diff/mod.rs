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
                old_path: "a".to_owned(),
                new_path: "b".to_owned(),
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
            old_path: "a".to_owned(),
            new_path: "b".to_owned(),
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

    /// Parse unified diff format — tries diffy first, falls back to legacy parser.
    pub fn parse(text: &str) -> Diff {
        // Extract paths before parsing (diffy doesn't expose them).
        let mut old_path = String::new();
        let mut new_path = String::new();
        for line in text.lines() {
            if line.starts_with("--- ") && line.len() > 4 {
                old_path = line[4..].to_string();
            } else if line.starts_with("+++ ") && line.len() > 4 {
                new_path = line[4..].to_string();
            }
            if !old_path.is_empty() && !new_path.is_empty() {
                break;
            }
        }
        let result = diffy::Patch::from_str(text);
        match result {
            Ok(p) => {
                let mut diff = diffy_to_canonical(&p);
                diff.old_path = old_path;
                diff.new_path = new_path;
                diff
            }
            Err(_) => legacy_parse_diff(text),
        }
    }
}

fn diffy_to_canonical(p: &diffy::Patch<str>) -> Diff {
    let hunks = p
        .hunks()
        .iter()
        .map(|h| {
            let old_r = h.old_range();
            let new_r = h.new_range();
            let mut lines = Vec::new();
            let mut ol = old_r.start() as u32;
            let mut nl = new_r.start() as u32;
            for l in h.lines() {
                match l {
                    diffy::Line::Delete(s) => {
                        let n = ol;
                        ol += 1;
                        lines.push(DiffLine::Removed((*s).to_string(), Some(n)));
                    }
                    diffy::Line::Insert(s) => {
                        let n = nl;
                        nl += 1;
                        lines.push(DiffLine::Added((*s).to_string(), Some(n)));
                    }
                    diffy::Line::Context(s) => {
                        ol += 1;
                        nl += 1;
                        lines.push(DiffLine::Context((*s).to_string()));
                    }
                }
            }
            DiffHunk {
                header: format!(
                    "@@ -{},{} +{},{} @@",
                    old_r.start(),
                    old_r.len(),
                    new_r.start(),
                    new_r.len()
                ),
                lines,
            }
        })
        .collect();
    Diff {
        old_path: "a".into(),
        new_path: "b".into(),
        hunks,
    }
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
            header: line.to_owned(),
            lines: Vec::new(),
        });
        // Add hunk header as a context line (preserves original behavior)
        self.push_line(DiffLine::Context(line.to_owned()));
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
mod tests;
