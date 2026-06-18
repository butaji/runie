//! Diff rendering for edit tool output.
//!
//! The TUI renders diffs using the canonical `runie_core::diff::Diff` type
//! directly where available, avoiding a string round-trip.

use patch::Line::{Add, Context, Remove};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::{
    color_accent, color_diff_insert_bg, color_diff_remove_bg, color_dim, color_fg, color_success,
};

// ── Canonical diff conversion ────────────────────────────────────────────────

/// Convert the canonical diff type to the TUI's internal representation.
fn canonical_to_parsed(canonical: &runie_core::diff::Diff) -> ParsedDiff {
    let old_path = Some(canonical.old_path.clone());
    let new_path = Some(canonical.new_path.clone());
    let lines: Vec<ParsedDiffLine> = canonical
        .hunks
        .iter()
        .flat_map(|hunk| {
            let mut out = Vec::with_capacity(hunk.lines.len() + 1);
            out.push(ParsedDiffLine {
                line_type: DiffLineType::HunkHeader,
                content: hunk.header.clone(),
                line_number: None,
            });
            for line in &hunk.lines {
                let (content, lt) = match line {
                    runie_core::diff::DiffLine::Added(s) => (s.clone(), DiffLineType::Added),
                    runie_core::diff::DiffLine::Removed(s) => (s.clone(), DiffLineType::Removed),
                    runie_core::diff::DiffLine::Context(s) => (s.clone(), DiffLineType::Context),
                };
                out.push(ParsedDiffLine {
                    line_type: lt,
                    content,
                    line_number: None,
                });
            }
            out
        })
        .collect();

    ParsedDiff {
        lines,
        old_path,
        new_path,
    }
}

/// Render a canonical diff to styled ratatui lines.
pub fn render_canonical_diff(
    diff: &runie_core::diff::Diff,
    gutter_width: usize,
) -> Vec<Line<'static>> {
    let parsed = canonical_to_parsed(diff);
    render_diff(&parsed, gutter_width)
}

// ── Legacy parsing (for imperfect agent output strings) ─────────────────────

/// A parsed line from a unified diff.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    /// File header (--- or +++)
    FileHeader,
    /// Hunk header (@@ ... @@)
    HunkHeader,
    /// Added line (+)
    Added,
    /// Removed line (-)
    Removed,
    /// Context line (unchanged)
    Context,
}

/// A single line with its type and content.
#[derive(Debug, Clone)]
pub struct ParsedDiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub line_number: Option<u32>,
}

/// Parsed diff with metadata.
#[derive(Debug, Clone)]
pub struct ParsedDiff {
    pub lines: Vec<ParsedDiffLine>,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

/// Check if text looks like a unified diff.
pub fn is_diff_output(text: &str) -> bool {
    let first_line = text.lines().next().unwrap_or("");
    first_line.starts_with("--- ") || first_line.starts_with("diff ")
}

/// Parse unified diff format — tries patch crate first, falls back to legacy parser.
pub fn parse_diff(text: &str) -> ParsedDiff {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        patch::Patch::from_single(text)
    }));
    if let Ok(Ok(p)) = result {
        return parse_patch(p);
    }
    legacy_parse_diff(text)
}

fn parse_patch(p: patch::Patch) -> ParsedDiff {
    let old_path = Some(p.old.path.to_string());
    let new_path = Some(p.new.path.to_string());
    let lines = parse_patch_hunks(&p.hunks);
    ParsedDiff {
        lines,
        old_path,
        new_path,
    }
}

fn parse_patch_hunks(hunks: &[patch::Hunk]) -> Vec<ParsedDiffLine> {
    let mut lines = Vec::new();
    for hunk in hunks {
        if let Some(hint) = hunk.hint() {
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::HunkHeader,
                content: hint.to_string(),
                line_number: None,
            });
        }
        lines.extend(parse_hunk_lines(hunk));
    }
    lines
}

fn parse_hunk_lines(hunk: &patch::Hunk) -> Vec<ParsedDiffLine> {
    let mut lines = Vec::new();
    let mut old_line = hunk.old_range.start as u32;
    let mut new_line = hunk.new_range.start as u32;
    for line in &hunk.lines {
        match line {
            Add(s) => {
                let num = new_line;
                new_line += 1;
                lines.push(ParsedDiffLine {
                    line_type: DiffLineType::Added,
                    content: s.to_string(),
                    line_number: Some(num),
                });
            }
            Remove(s) => {
                let num = old_line;
                old_line += 1;
                lines.push(ParsedDiffLine {
                    line_type: DiffLineType::Removed,
                    content: s.to_string(),
                    line_number: Some(num),
                });
            }
            Context(s) => {
                let num = old_line;
                old_line += 1;
                new_line += 1;
                lines.push(ParsedDiffLine {
                    line_type: DiffLineType::Context,
                    content: s.to_string(),
                    line_number: Some(num),
                });
            }
        }
    }
    lines
}

// ── Legacy parser for imperfect diffs ────────────────────────────────────────

fn legacy_parse_diff(text: &str) -> ParsedDiff {
    let mut state = LegacyParseState::default();
    for line in text.lines() {
        state.parse_line(line);
    }
    ParsedDiff {
        lines: state.lines,
        old_path: state.old_path,
        new_path: state.new_path,
    }
}

#[derive(Default)]
struct LegacyParseState {
    lines: Vec<ParsedDiffLine>,
    old_path: Option<String>,
    new_path: Option<String>,
    old_line_num: Option<u32>,
    new_line_num: Option<u32>,
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
        self.push_line(DiffLineType::FileHeader, line.to_string(), None);
    }

    fn parse_new_header(&mut self, line: &str) {
        self.new_path = Some(line[4..].to_string());
        self.push_line(DiffLineType::FileHeader, line.to_string(), None);
    }

    fn parse_added(&mut self, line: &str) {
        let num = self.new_line_num;
        if let Some(ref mut n) = self.new_line_num {
            *n += 1;
        }
        self.push_line(DiffLineType::Added, line[1..].to_string(), num);
    }

    fn parse_removed(&mut self, line: &str) {
        let num = self.old_line_num;
        if let Some(ref mut n) = self.old_line_num {
            *n += 1;
        }
        self.push_line(DiffLineType::Removed, line[1..].to_string(), num);
    }

    fn parse_context(&mut self, line: &str) {
        if let Some(ref mut o) = self.old_line_num {
            *o += 1;
        }
        if let Some(ref mut n) = self.new_line_num {
            *n += 1;
        }
        self.push_line(
            DiffLineType::Context,
            line[1..].to_string(),
            self.old_line_num,
        );
    }

    fn parse_hunk_header(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        self.old_line_num = parts
            .get(1)
            .and_then(|s| s.split(',').next()?.strip_prefix('-')?.parse().ok());
        self.new_line_num = parts
            .get(2)
            .and_then(|s| s.split(',').next()?.strip_prefix('+')?.parse().ok());
        self.push_line(DiffLineType::HunkHeader, line.to_string(), None);
    }

    fn push_line(&mut self, lt: DiffLineType, content: String, num: Option<u32>) {
        self.lines.push(ParsedDiffLine {
            line_type: lt,
            content,
            line_number: num,
        });
    }
}

// ── Styling ───────────────────────────────────────────────────────────────────

/// Style for a diff line based on its type.
pub fn diff_line_style(line_type: &DiffLineType) -> Style {
    match line_type {
        DiffLineType::Added => Style::default()
            .fg(color_success())
            .bg(color_diff_insert_bg()),
        DiffLineType::Removed => Style::default().fg(Color::Red).bg(color_diff_remove_bg()),
        DiffLineType::HunkHeader => Style::default()
            .fg(color_accent())
            .add_modifier(Modifier::BOLD),
        DiffLineType::FileHeader => Style::default().fg(color_dim()),
        DiffLineType::Context => Style::default().fg(color_fg()),
    }
}

/// Prefix character for a diff line.
pub fn diff_line_prefix(line_type: &DiffLineType) -> &'static str {
    match line_type {
        DiffLineType::Added => "+",
        DiffLineType::Removed => "-",
        DiffLineType::Context => " ",
        DiffLineType::HunkHeader => "",
        DiffLineType::FileHeader => "",
    }
}

/// Render a parsed diff to styled ratatui lines.
pub fn render_diff(diff: &ParsedDiff, gutter_width: usize) -> Vec<Line<'static>> {
    let mut output = Vec::new();

    for parsed in &diff.lines {
        let prefix = diff_line_prefix(&parsed.line_type);
        let style = diff_line_style(&parsed.line_type);

        let line_num_str = match parsed.line_number {
            Some(n) => format!("{:>width$}", n, width = gutter_width),
            None => " ".repeat(gutter_width),
        };

        let spans: Vec<Span<'static>> = vec![
            Span::styled(
                line_num_str,
                Style::default()
                    .fg(color_dim())
                    .bg(style.bg.unwrap_or(Color::Reset)),
            ),
            Span::styled(prefix, style),
            Span::styled(parsed.content.clone(), style),
        ];

        output.push(Line::from(spans));
    }

    output
}

/// Render diff text directly (convenience — for non-canonical tool output).
pub fn render_diff_text(text: &str) -> Vec<Line<'static>> {
    if !is_diff_output(text) {
        return text.lines().map(|l| Line::from(l.to_string())).collect();
    }

    let diff = parse_diff(text);
    let gutter_width = 4;
    render_diff(&diff, gutter_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_diff_output() {
        let diff = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n line1\n-old\n+new\n line3";
        assert!(is_diff_output(diff));
    }

    #[test]
    fn rejects_non_diff_output() {
        let text = "Hello, this is regular text";
        assert!(!is_diff_output(text));
    }

    #[test]
    fn parses_simple_diff() {
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1,3 +1,4 @@\n line1\n-old\n+new\n line3";
        let parsed = parse_diff(diff);

        assert_eq!(parsed.old_path, Some("a/test.txt".to_string()));
        assert_eq!(parsed.new_path, Some("b/test.txt".to_string()));
        assert!(!parsed.lines.is_empty());

        let added = parsed
            .lines
            .iter()
            .find(|l| l.line_type == DiffLineType::Added);
        assert!(added.is_some());
        assert_eq!(added.unwrap().content, "new");

        let removed = parsed
            .lines
            .iter()
            .find(|l| l.line_type == DiffLineType::Removed);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content, "old");
    }

    #[test]
    fn parses_hunk_header() {
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1,5 +1,7 @@ context";
        let parsed = parse_diff(diff);
        assert!(!parsed.lines.is_empty());
        assert!(parsed
            .lines
            .iter()
            .any(|l| matches!(l.line_type, DiffLineType::HunkHeader)));
    }

    #[test]
    fn diff_line_styles() {
        // Force truecolor so quantized approximations do not break RGB assertions.
        crate::theme::set_current_theme_with_caps(
            crate::theme::DEFAULT_THEME_NAME,
            crate::terminal::caps::TerminalCapabilities {
                truecolor: true,
                ..crate::terminal::caps::TerminalCapabilities::default()
            },
        );

        assert_eq!(
            diff_line_style(&DiffLineType::Added).fg,
            Some(color_success())
        );
        // Added lines now carry an insert (green) background.
        assert_ne!(diff_line_style(&DiffLineType::Added).bg, None);

        assert_eq!(diff_line_style(&DiffLineType::Removed).fg, Some(Color::Red));
        // Removed lines now carry a remove (red) background.
        assert_ne!(diff_line_style(&DiffLineType::Removed).bg, None);

        assert_eq!(diff_line_style(&DiffLineType::Context).fg, Some(color_fg()));
    }

    #[test]
    fn diff_line_prefixes() {
        assert_eq!(diff_line_prefix(&DiffLineType::Added), "+");
        assert_eq!(diff_line_prefix(&DiffLineType::Removed), "-");
        assert_eq!(diff_line_prefix(&DiffLineType::Context), " ");
    }

    #[test]
    fn render_diff_output() {
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1,1 +1,1 @@\n-old\n+new";
        let lines = render_diff_text(diff);

        assert!(!lines.is_empty());
        for line in &lines {
            assert!(!line.spans.is_empty());
        }
    }

    #[test]
    fn render_non_diff_as_plain() {
        let text = "This is not a diff";
        let lines = render_diff_text(text);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "This is not a diff");
    }

    #[test]
    fn empty_content() {
        let diff = "";
        let lines = render_diff_text(diff);
        assert!(lines.is_empty());
    }

    #[test]
    fn tui_renders_canonical_diff() {
        // Canonical diff rendered directly produces the same styled output as parsed text.
        let canonical = runie_core::diff::Diff::generate("old", "new");
        let from_canonical = render_canonical_diff(&canonical, 4);
        let from_text = render_diff_text("---\n+++ \n@@ -1 +1 @@\n-old\n+new");

        // Both should have non-empty styled output.
        assert!(!from_canonical.is_empty());
        assert!(!from_text.is_empty());
    }
}
