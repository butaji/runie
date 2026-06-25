//! Diff rendering for edit tool output.
//!
//! The TUI renders diffs using the canonical `runie_core::diff::Diff` type
//! directly where available, avoiding a string round-trip.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::theme::{
    color_diff_insert_bg, color_diff_remove_bg, color_dim, color_fg, color_success,
};

use runie_core::diff::{Diff, DiffLine};

// ── Rendering canonical diff ──────────────────────────────────────────────────

/// Render a canonical diff to styled ratatui lines.
pub fn render_canonical_diff(diff: &Diff, gutter_width: usize) -> Vec<Line<'static>> {
    let mut output = Vec::new();

    for hunk in &diff.hunks {
        // Hunk lines
        for line in &hunk.lines {
            let prefix = diff_line_prefix(line);
            let style = diff_line_style(line);

            // Only add gutter if line number is present
            let spans: Vec<Span<'static>> = match line.line_number() {
                Some(n) => {
                    let line_num_str = format!("{:>width$}", n, width = gutter_width);
                    vec![
                        Span::styled(
                            line_num_str,
                            Style::default()
                                .fg(color_dim())
                                .bg(style.bg.unwrap_or(Color::Reset)),
                        ),
                        Span::styled(prefix, style),
                        Span::styled(line.content().to_string(), style),
                    ]
                }
                None => vec![
                    Span::styled(prefix, style),
                    Span::styled(line.content().to_string(), style),
                ],
            };
            output.push(Line::from(spans));
        }
    }

    output
}

// ── Styling ──────────────────────────────────────────────────────────────────

/// Style for a diff line based on its type.
pub fn diff_line_style(line: &DiffLine) -> Style {
    match line {
        DiffLine::Added(_, _) => Style::default()
            .fg(color_success())
            .bg(color_diff_insert_bg()),
        DiffLine::Removed(_, _) => Style::default().fg(Color::Red).bg(color_diff_remove_bg()),
        DiffLine::Context(_) => Style::default().fg(color_fg()),
    }
}

/// Prefix character for a diff line.
pub fn diff_line_prefix(line: &DiffLine) -> &'static str {
    match line {
        DiffLine::Added(_, _) => "+",
        DiffLine::Removed(_, _) => "-",
        DiffLine::Context(_) => " ",
    }
}

// ── Text rendering ────────────────────────────────────────────────────────────

/// Render diff text directly (convenience — for non-canonical tool output).
pub fn render_diff_text(text: &str) -> Vec<Line<'static>> {
    if !Diff::is_diff_output(text) {
        return text.lines().map(|l| Line::from(l.to_string())).collect();
    }

    let diff = Diff::parse(text);
    render_canonical_diff(&diff, 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_diff_output() {
        let diff = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n line1\n-old\n+new\n line3";
        assert!(Diff::is_diff_output(diff));
    }

    #[test]
    fn rejects_non_diff_output() {
        let text = "Hello, this is regular text";
        assert!(!Diff::is_diff_output(text));
    }

    #[test]
    fn parses_simple_diff() {
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1,3 +1,4 @@\n line1\n-old\n+new\n line3";
        let parsed = Diff::parse(diff);

        assert_eq!(parsed.old_path, "a/test.txt");
        assert_eq!(parsed.new_path, "b/test.txt");
        assert!(!parsed.hunks.is_empty());

        let added = parsed
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .find(|l| matches!(l, DiffLine::Added(_, _)));
        assert!(added.is_some());
        assert_eq!(added.unwrap().content(), "new");

        let removed = parsed
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .find(|l| matches!(l, DiffLine::Removed(_, _)));
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content(), "old");
    }

    #[test]
    fn parses_hunk_header() {
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1,5 +1,7 @@ context";
        let parsed = Diff::parse(diff);
        assert!(!parsed.hunks.is_empty());
        assert!(parsed
            .hunks
            .iter()
            .any(|h| !h.header.is_empty()));
    }

    #[test]
    fn diff_line_styles() {
        // Force truecolor so quantized approximations do not break RGB assertions.
        crate::theme::set_current_theme_with_caps(
            crate::theme::DEFAULT_THEME_NAME,
            crate::terminal::caps::TerminalCapabilities {
                truecolor: true,
                ..Default::default()
            },
        );

        let added = DiffLine::Added("test".to_string(), Some(1));
        assert_eq!(diff_line_style(&added).fg, Some(color_success()));
        assert_ne!(diff_line_style(&added).bg, None);

        let removed = DiffLine::Removed("test".to_string(), Some(1));
        assert_eq!(diff_line_style(&removed).fg, Some(Color::Red));
        assert_ne!(diff_line_style(&removed).bg, None);

        let context = DiffLine::Context("test".to_string());
        assert_eq!(diff_line_style(&context).fg, Some(color_fg()));
    }

    #[test]
    fn diff_line_prefixes() {
        let added = DiffLine::Added("test".to_string(), None);
        let removed = DiffLine::Removed("test".to_string(), None);
        let context = DiffLine::Context("test".to_string());

        assert_eq!(diff_line_prefix(&added), "+");
        assert_eq!(diff_line_prefix(&removed), "-");
        assert_eq!(diff_line_prefix(&context), " ");
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
        // Canonical diff rendered directly produces styled output.
        let canonical = Diff::generate("old", "new");
        let from_canonical = render_canonical_diff(&canonical, 4);

        // Should have non-empty styled output.
        assert!(!from_canonical.is_empty());
    }

    // ── Layer 3: Rendering tests ────────────────────────────────────────────────

    #[test]
    fn render_canonical_diff_unchanged() {
        use ratatui::backend::TestBackend;
        use ratatui::layout::Rect;
        use ratatui::widgets::{Paragraph, Widget};
        use ratatui::Terminal;

        let canonical = Diff::generate("line1\nold\nline3", "line1\nnew\nline3");
        let lines = render_canonical_diff(&canonical, 4);

        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                Paragraph::new(lines.clone())
                    .render(Rect::new(0, 0, 40, 20), f.buffer_mut());
            })
            .unwrap();

        // Verify we rendered something (buffer should have non-empty content)
        let buffer = terminal.backend().buffer();
        let height = buffer.area.height;
        let mut has_content = false;
        for y in 0..height {
            for x in 0..40 {
                let cell = &buffer[(x, y)];
                if !cell.symbol().is_empty() && cell.symbol() != " " {
                    has_content = true;
                    break;
                }
            }
            if has_content {
                break;
            }
        }
        assert!(has_content, "Buffer should contain non-whitespace content");
    }

    #[test]
    fn parse_diff_still_parses_unified_diff() {
        let diff = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n line1\n-old_line\n+new_line\n line3";
        let parsed = Diff::parse(diff);

        assert_eq!(parsed.old_path, "a/src/main.rs");
        assert_eq!(parsed.new_path, "b/src/main.rs");
        assert!(!parsed.hunks.is_empty());
    }
}
