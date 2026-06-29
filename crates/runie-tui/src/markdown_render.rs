//! Markdown parsing for agent messages.
//!
//! Uses `tui_markdown` for rendering markdown to styled text. This replaces
//! the custom inline styling with the library's implementation.

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::text::Text;

pub use runie_core::markdown::{extract_code_blocks, parse_inline_spans, CodeBlock, MdInline};

use crate::theme::{color_accent, color_code_bg};

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: ratatui::style::Style,
}

/// Apply a base color to pre-parsed inline spans from core's unified AST,
/// producing styled `MdSpan`s for rendering.
pub fn apply_color_to_inlines(inlines: &[MdInline], base_color: Color) -> Vec<MdSpan> {
    let base = ratatui::style::Style::default().fg(base_color);
    let code_style = ratatui::style::Style::default()
        .fg(color_accent())
        .bg(color_code_bg());
    let mut spans = Vec::new();
    let style_stack: Vec<ratatui::style::Style> = vec![base];

    for inline in inlines {
        match inline {
            MdInline::Text(s) => push_span(&mut spans, s, *style_stack.last().unwrap()),
            MdInline::Bold(s) => {
                let bold = style_stack.last().unwrap().add_modifier(Modifier::BOLD);
                push_span(&mut spans, s, bold);
            }
            MdInline::Italic(s) => {
                let italic = style_stack.last().unwrap().add_modifier(Modifier::ITALIC);
                push_span(&mut spans, s, italic);
            }
            MdInline::Code(s) => push_span(&mut spans, s, code_style),
            MdInline::Strike(s) => {
                let strike = style_stack
                    .last()
                    .unwrap()
                    .add_modifier(Modifier::CROSSED_OUT);
                push_span(&mut spans, s, strike);
            }
            MdInline::SoftBreak | MdInline::HardBreak => {
                push_span(&mut spans, "\n", *style_stack.last().unwrap());
            }
        }
    }
    spans
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
    spans.push(MdSpan {
        content: text.to_owned(),
        style,
    });
}

/// Parse inline markdown into styled spans using `tui_markdown`.
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    // Use tui_markdown to parse and style the inline markdown
    let text: Text<'_> = tui_markdown::from_str(text);
    text_to_md_spans(&text)
}

/// Parse inline markdown with a custom base foreground color.
/// Note: tui_markdown doesn't support custom base colors, so this falls back
/// to the core parsing with the custom color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    apply_color_to_inlines(&parse_inline_spans(text), base_color)
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
            result.push(MdSpan {
                content: span.content.to_string(),
                style: span.style,
            });
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
        let inlines = vec![
            MdInline::Text("plain ".into()),
            MdInline::Bold("bold".into()),
            MdInline::Text(" ".into()),
            MdInline::Italic("italic".into()),
            MdInline::Text(" ".into()),
            MdInline::Code("code".into()),
        ];
        let spans = apply_color_to_inlines(&inlines, Color::White);

        let has_bold = spans
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(Modifier::BOLD) == s.style);
        let has_italic = spans
            .iter()
            .any(|s| s.content == "italic" && s.style.add_modifier(Modifier::ITALIC) == s.style);
        let has_code = spans
            .iter()
            .any(|s| s.content == "code" && s.style.bg.is_some());
        assert!(has_bold, "missing bold span");
        assert!(has_italic, "missing italic span");
        assert!(has_code, "missing code span");

        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let line = Line::from(md_to_spans(&spans));
                f.render_widget(Paragraph::new(line), f.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let first_line = buf.content.chunks(30).next().unwrap();
        let text: String = first_line.iter().map(|c| c.symbol()).collect();
        assert!(
            text.contains("plain bold italic code"),
            "rendered text missing: {text}"
        );
    }

    #[test]
    fn parse_inline_markdown_uses_tui_markdown() {
        // Test that parse_inline_markdown produces styled spans via tui_markdown
        let result = parse_inline_markdown("This is **bold** and *italic*.");
        assert!(!result.is_empty());
        // Bold text should have the BOLD modifier
        let has_bold = result
            .iter()
            .any(|s| s.content == "bold" && s.style.add_modifier(Modifier::BOLD) == s.style);
        assert!(has_bold, "bold span should have BOLD modifier");
    }

    #[test]
    fn parse_inline_markdown_with_color_falls_back_to_core() {
        let custom_color = Color::Red;
        let result = parse_inline_markdown_with_color("Plain text.", custom_color);
        // The base color should be applied
        assert!(!result.is_empty());
    }
}
