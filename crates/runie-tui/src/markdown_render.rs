//! Markdown rendering for agent messages.
//!
//! Uses `tui_markdown` for all markdown parsing and styling. This replaces
//! the former custom inline AST (`MdInline`) with the library's implementation.

use ratatui::style::Color;
use ratatui::text::Span;
use ratatui::text::Text;

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: ratatui::style::Style,
}

/// Parse markdown text and style it using `tui_markdown`, then apply a base
/// foreground color to all spans.
///
/// Handles literal newlines by splitting the text first, styling each line
/// with `tui_markdown`, and recombining with explicit `\n` spans. This
/// preserves the same line-boundary behavior as the former `parse_inline_spans`
/// approach where `SoftBreak` spans were inserted for each `\n`.
///
/// This replaces the former `MdInline[]` → `MdSpan[]` pipeline.
pub fn apply_color_to_inlines(text: &str, base_color: Color) -> Vec<MdSpan> {
    // Split by explicit newlines first. Each segment is styled independently with
    // tui_markdown, then rejoined with `\n` spans so downstream wrapping
    // (wrap_styled_spans::split_spans_by_newline) sees the same structure as
    // the old SoftBreak-span approach.
    let mut result = Vec::new();
    let mut is_first = true;
    for segment in text.split_inclusive('\n') {
        if !is_first {
            // Insert explicit newline span between segments
            push_span(&mut result, "\n", ratatui::style::Style::default());
        }
        if !segment.is_empty() && segment != "\n" {
            let parsed: Text<'_> = tui_markdown::from_str(segment.trim_end_matches('\n'));
            let raw = text_to_md_spans(&parsed);
            result.extend(override_base_color(raw, base_color));
        }
        is_first = false;
    }
    result
}

/// Parse markdown text into styled spans using `tui_markdown`.
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    let text: Text<'_> = tui_markdown::from_str(text);
    text_to_md_spans(&text)
}

/// Parse markdown text with a custom base foreground color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    apply_color_to_inlines(text, base_color)
}

/// Override the base foreground color on all spans while preserving modifiers
/// (bold, italic, etc.) and background colors.
fn override_base_color(spans: Vec<MdSpan>, base_color: Color) -> Vec<MdSpan> {
    spans
        .into_iter()
        .map(|s| {
            let modifier = s.style.add_modifier;
            let bg = s.style.bg;
            let mut new_style = ratatui::style::Style::default().fg(base_color);
            if modifier != Default::default() {
                new_style = new_style.add_modifier(modifier);
            }
            if let Some(b) = bg {
                new_style = new_style.bg(b);
            }
            MdSpan { content: s.content, style: new_style }
        })
        .collect()
}

fn push_span(spans: &mut Vec<MdSpan>, text: &str, style: ratatui::style::Style) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.content.push_str(text);
            return;
        }
    }
    spans.push(MdSpan { content: text.to_owned(), style });
}

/// Convert tui_markdown's Text output to our MdSpan format.
fn text_to_md_spans(text: &Text<'_>) -> Vec<MdSpan> {
    let mut result = Vec::new();
    for line in text.lines.iter() {
        if !result.is_empty() {
            // Add newline between lines
            push_span(&mut result, "\n", ratatui::style::Style::default());
        }
        for span in line.spans.iter() {
            result.push(MdSpan { content: span.content.to_string(), style: span.style });
        }
    }
    // Merge adjacent spans with the same style
    merge_adjacent_spans(result)
}

/// Merge adjacent spans with the same style to reduce span count.
fn merge_adjacent_spans(spans: Vec<MdSpan>) -> Vec<MdSpan> {
    if spans.is_empty() {
        return spans;
    }
    let mut result = Vec::new();
    let mut current = spans[0].clone();

    for span in spans.iter().skip(1) {
        if current.style == span.style {
            current.content.push_str(&span.content);
        } else {
            result.push(current);
            current = span.clone();
        }
    }
    result.push(current);
    result
}

/// Convert `MdSpan` slices to ratatui `Span`s.
pub fn md_to_spans(md_spans: &[MdSpan]) -> Vec<Span<'static>> {
    md_spans
        .iter()
        .map(|s| Span::styled(s.content.clone(), s.style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    #[test]
    fn styled_spans_preserved() {
        // Parse markdown text with inline styles and verify spans are extracted correctly.
        // Uses tui_markdown internally (via apply_color_to_inlines).
        let spans = apply_color_to_inlines("plain **bold** *italic* `code`", Color::White);

        let has_bold = spans
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        let has_italic = spans
            .iter()
            .any(|s| s.content == "italic" && s.style.add_modifier(ratatui::style::Modifier::ITALIC) == s.style);
        let has_code = spans
            .iter()
            .any(|s| s.content == "code" && s.style.bg.is_some());
        assert!(has_bold, "missing bold span");
        assert!(has_italic, "missing italic span");
        assert!(has_code, "missing code span");

        let backend = TestBackend::new(50, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let line = Line::from(md_to_spans(&spans));
                f.render_widget(Paragraph::new(line), f.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let first_line = buf.content.chunks(50).next().unwrap();
        let text: String = first_line.iter().map(|c| c.symbol()).collect();
        assert!(
            text.contains("plain bold italic code"),
            "rendered text missing: {text}"
        );
    }

    #[test]
    fn parse_inline_markdown_uses_tui_markdown() {
        // Test that parse_inline_markdown produces styled spans via tui_markdown.
        let result = parse_inline_markdown("This is **bold** and *italic*.");
        assert!(!result.is_empty());
        let has_bold = result
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        assert!(has_bold, "bold span should have BOLD modifier");
    }

    #[test]
    fn parse_inline_markdown_with_color_applies_base_color() {
        let custom_color = Color::Red;
        let result = parse_inline_markdown_with_color("Plain text.", custom_color);
        assert!(!result.is_empty());
        // All spans should have the base color applied.
        for span in &result {
            assert!(
                span.style.fg.is_some(),
                "span should have foreground color set"
            );
        }
    }

    #[test]
    fn apply_color_to_inlines_uses_tui_markdown() {
        // Verify apply_color_to_inlines uses tui_markdown by checking that it
        // correctly parses bold/italic from raw markdown text.
        let spans = apply_color_to_inlines("**strong** and *emphasis*", Color::Green);
        let has_strong = spans
            .iter()
            .any(|s| s.content == "strong" && s.style.add_modifier(ratatui::style::Modifier::BOLD) == s.style);
        let has_emphasis = spans
            .iter()
            .any(|s| s.content == "emphasis" && s.style.add_modifier(ratatui::style::Modifier::ITALIC) == s.style);
        assert!(has_strong, "missing strong span via tui_markdown");
        assert!(has_emphasis, "missing emphasis span via tui_markdown");
    }
}
