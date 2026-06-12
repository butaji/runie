//! Diff rendering for edit tool output
//!
//! Parses unified diff format and renders with syntax highlighting:
//! - Added lines: green
//! - Removed lines: red
//! - Context lines: default color
//! - Line numbers: dim

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::{color_accent, color_dim, color_fg, color_success};

/// A parsed line from a unified diff
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

/// A single line with its type and content
#[derive(Debug, Clone)]
pub struct ParsedDiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub line_number: Option<u32>,
}

/// Parsed diff with metadata
#[derive(Debug, Clone)]
pub struct ParsedDiff {
    pub lines: Vec<ParsedDiffLine>,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

/// Check if text looks like a unified diff
pub fn is_diff_output(text: &str) -> bool {
    let first_line = text.lines().next().unwrap_or("");
    first_line.starts_with("--- ") || first_line.starts_with("diff ")
}

/// Parse unified diff format
pub fn parse_diff(text: &str) -> ParsedDiff {
    let mut lines = Vec::new();
    let mut old_path = None;
    let mut new_path = None;
    let mut old_line_num: Option<u32> = None;
    let mut new_line_num: Option<u32> = None;

    for line in text.lines() {
        if line.starts_with("--- ") {
            old_path = Some(line.trim_start_matches("--- ").to_string());
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::FileHeader,
                content: line.to_string(),
                line_number: None,
            });
        } else if line.starts_with("+++ ") {
            new_path = Some(line.trim_start_matches("+++ ").to_string());
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::FileHeader,
                content: line.to_string(),
                line_number: None,
            });
        } else if line.starts_with("@@ ") {
            // Parse hunk header: @@ -start,count +start,count @@
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let ranges: Vec<&str> = parts[1].split(',').collect();
                if ranges[0].starts_with('-') {
                    old_line_num = ranges[0].trim_start_matches('-').parse().ok();
                }
                let new_parts: Vec<&str> = parts.get(2).unwrap_or(&"").split(',').collect();
                if new_parts[0].starts_with('+') {
                    new_line_num = Some(new_parts[0].trim_start_matches('+').parse().unwrap_or(1));
                }
            }
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::HunkHeader,
                content: line.to_string(),
                line_number: None,
            });
        } else if line.starts_with('+') && !line.starts_with("+++") {
            let num = new_line_num.take();
            if let Some(ref mut n) = new_line_num {
                *n += 1;
            }
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::Added,
                content: line[1..].to_string(),
                line_number: num,
            });
        } else if line.starts_with('-') && !line.starts_with("---") {
            let num = old_line_num.take();
            if let Some(ref mut n) = old_line_num {
                *n += 1;
            }
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::Removed,
                content: line[1..].to_string(),
                line_number: num,
            });
        } else if line.starts_with(' ') {
            let num = match (&old_line_num, &new_line_num) {
                (Some(o), Some(_n)) => Some(*o),
                (Some(o), None) => Some(*o),
                (None, Some(n)) => Some(*n),
                (None, None) => None,
            };
            if let Some(ref mut o) = old_line_num {
                *o += 1;
            }
            if let Some(ref mut n) = new_line_num {
                *n += 1;
            }
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::Context,
                content: line.strip_prefix(' ').unwrap_or(line).to_string(),
                line_number: num,
            });
        } else if line.is_empty() {
            // Empty line in diff content
            lines.push(ParsedDiffLine {
                line_type: DiffLineType::Context,
                content: String::new(),
                line_number: None,
            });
        }
        // Skip any other lines (like \ No newline at end of file)
    }

    ParsedDiff {
        lines,
        old_path,
        new_path,
    }
}

/// Style for a diff line based on its type
pub fn diff_line_style(line_type: &DiffLineType) -> Style {
    match line_type {
        DiffLineType::Added => Style::default().fg(color_success()),
        DiffLineType::Removed => Style::default().fg(Color::Red),
        DiffLineType::HunkHeader => Style::default()
            .fg(color_accent())
            .add_modifier(Modifier::BOLD),
        DiffLineType::FileHeader => Style::default().fg(color_dim()),
        DiffLineType::Context => Style::default().fg(color_fg()),
    }
}

/// Prefix character for a diff line
pub fn diff_line_prefix(line_type: &DiffLineType) -> &'static str {
    match line_type {
        DiffLineType::Added => "+",
        DiffLineType::Removed => "-",
        DiffLineType::Context => " ",
        DiffLineType::HunkHeader => "",
        DiffLineType::FileHeader => "",
    }
}

/// Render a parsed diff to styled ratatui lines
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
            Span::styled(line_num_str, Style::default().fg(color_dim())),
            Span::styled(prefix, style),
            Span::styled(parsed.content.clone(), style),
        ];

        output.push(Line::from(spans));
    }

    output
}

/// Render diff text directly (convenience function)
pub fn render_diff_text(text: &str) -> Vec<Line<'static>> {
    if !is_diff_output(text) {
        // Not a diff, return as plain text
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

        // Find added line
        let added = parsed
            .lines
            .iter()
            .find(|l| l.line_type == DiffLineType::Added);
        assert!(added.is_some());
        assert_eq!(added.unwrap().content, "new");

        // Find removed line
        let removed = parsed
            .lines
            .iter()
            .find(|l| l.line_type == DiffLineType::Removed);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content, "old");
    }

    #[test]
    fn parses_hunk_header() {
        let diff = "@@ -1,5 +1,7 @@ context";
        let parsed = parse_diff(diff);

        assert_eq!(parsed.lines.len(), 1);
        assert!(matches!(
            parsed.lines[0].line_type,
            DiffLineType::HunkHeader
        ));
    }

    #[test]
    fn diff_line_styles() {
        assert_eq!(
            diff_line_style(&DiffLineType::Added).fg,
            Some(color_success())
        );
        assert_eq!(diff_line_style(&DiffLineType::Removed).fg, Some(Color::Red));
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
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-old\n+new";
        let lines = render_diff_text(diff);

        assert!(!lines.is_empty());
        // Each line should have spans
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
    fn preserves_line_numbers() {
        let diff = "@@ -10,3 +10,4 @@\n context\n-old\n+new\n+added";
        let parsed = parse_diff(diff);

        let removed = parsed
            .lines
            .iter()
            .find(|l| l.line_type == DiffLineType::Removed);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().line_number, Some(11));
    }
}
