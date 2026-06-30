//! Canonical diff representation shared by agent and TUI.
//!
//! The agent generates a `Diff` directly; the TUI renders it without parsing.
//! String serialization is only used for copy/export.

use similar::{Algorithm, ChangeTag, TextDiff};
use std::time::Duration;

const DIFF_DEADLINE_SECS: u64 = 5;

/// Normalize diff content lines: ensure proper unified diff prefix.
fn normalize_content_line(line: &str) -> Option<(char, &str)> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.chars().next()? {
        '+' => Some(('+', &trimmed[1..])),
        '-' => Some(('-', &trimmed[1..])),
        ' ' => Some((' ', &trimmed[1..])),
        _ => Some((' ', trimmed)), // Non-standard: treat as context
    }
}

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

        // Build hunks directly from similar changes (replaces HunkBuilder)
        let mut hunks = Vec::new();
        let mut current: Vec<DiffLine> = Vec::new();
        let mut old_start = 1;
        let mut new_start = 1;
        let mut old_len = 0;
        let mut new_len = 0;
        let mut old_line = 1;
        let mut new_line = 1;
        let mut in_hunk = false;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    if in_hunk {
                        finish_hunk(&mut hunks, &mut current, old_start, old_len, new_start, new_len);
                        in_hunk = false;
                        old_len = 0;
                        new_len = 0;
                    }
                    old_line += 1;
                    new_line += 1;
                }
                ChangeTag::Delete => {
                    if !in_hunk {
                        old_start = old_line;
                        new_start = new_line;
                        in_hunk = true;
                    }
                    current.push(DiffLine::Removed(
                        change.value().trim_end().to_string(),
                        Some(old_line as u32),
                    ));
                    old_len += 1;
                    old_line += 1;
                }
                ChangeTag::Insert => {
                    if !in_hunk {
                        old_start = old_line;
                        new_start = new_line;
                        in_hunk = true;
                    }
                    current.push(DiffLine::Added(
                        change.value().trim_end().to_string(),
                        Some(new_line as u32),
                    ));
                    new_len += 1;
                    new_line += 1;
                }
            }
        }
        if in_hunk {
            finish_hunk(&mut hunks, &mut current, old_start, old_len, new_start, new_len);
        }

        Diff {
            old_path: "a".to_owned(),
            new_path: "b".to_owned(),
            hunks,
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
            Err(_) => fallback_parse_diff(text),
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
/// Finish a hunk and add to the list.
fn finish_hunk(
    hunks: &mut Vec<DiffHunk>,
    lines: &mut Vec<DiffLine>,
    old_start: usize,
    old_len: usize,
    new_start: usize,
    new_len: usize,
) {
    if lines.is_empty() {
        return;
    }
    hunks.push(DiffHunk {
        header: format!("@@ -{},{} +{},{} @@", old_start, old_len, new_start, new_len),
        lines: std::mem::take(lines),
    });
}

/// Minimal fallback parser for imperfect agent output that diffy rejects.
/// Does not validate hunk line counts; parses content as-is.
fn fallback_parse_diff(text: &str) -> Diff {
    let mut old_path = String::new();
    let mut new_path = String::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut hunks = Vec::new();
    let mut in_hunk = false;

    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }

        // Parse headers
        if let Some(rest) = trimmed.strip_prefix("--- ") {
            old_path = rest.to_string();
        } else if let Some(rest) = trimmed.strip_prefix("+++ ") {
            new_path = rest.to_string();
        } else if let Some(header) = trimmed.strip_prefix("@@ ") {
            // Flush previous hunk
            if let Some(hunk) = current_hunk.take() {
                if !hunk.lines.is_empty() {
                    hunks.push(hunk);
                }
            }
            // Start new hunk with header as context line (preserves original behavior)
            current_hunk = Some(DiffHunk {
                header: format!("@@ {}", header),
                lines: vec![DiffLine::Context(format!("@@ {}", header))],
            });
            in_hunk = true;
        } else if in_hunk {
            // Parse hunk content
            if let Some((prefix, content)) = normalize_content_line(trimmed) {
                let line = match prefix {
                    '+' => DiffLine::Added(content.to_string(), None),
                    '-' => DiffLine::Removed(content.to_string(), None),
                    _ => DiffLine::Context(content.to_string()),
                };
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(line);
                }
            }
        }
    }

    // Flush final hunk
    if let Some(hunk) = current_hunk {
        if !hunk.lines.is_empty() {
            hunks.push(hunk);
        }
    }

    Diff { old_path, new_path, hunks }
}
