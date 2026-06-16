//! Markdown parsing for agent messages.
//!
//! Uses the unified AST from `runie_core::markdown`. Inline spans are parsed
//! once by core and reused here for styling — no double-parsing.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub use runie_core::markdown::{extract_code_blocks, parse_inline_spans, CodeBlock, MdInline};

use crate::theme::{color_accent, color_code_bg, color_fg_bright};

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: Style,
}

/// Apply a base color to pre-parsed inline spans from core's unified AST,
/// producing styled `MdSpan`s for rendering. This avoids re-parsing the text.
pub fn apply_color_to_inlines(inlines: &[MdInline], base_color: Color) -> Vec<MdSpan> {
    let base = Style::default().fg(base_color);
    let code_style = Style::default().fg(color_accent()).bg(color_code_bg());
    let mut spans = Vec::new();
    let mut style_stack: Vec<Style> = vec![base];

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
                let strike = style_stack.last().unwrap().add_modifier(Modifier::CROSSED_OUT);
                push_span(&mut spans, s, strike);
            }
            MdInline::SoftBreak | MdInline::HardBreak => {
                push_span(&mut spans, "\n", *style_stack.last().unwrap());
            }
        }
    }
    spans
}

fn push_span(spans: &mut Vec<MdSpan>, text: &str, style: Style) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.content.push_str(text);
            return;
        }
    }
    spans.push(MdSpan { content: text.to_string(), style });
}

/// Parse inline markdown into styled spans (delegates to core + color application).
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    apply_color_to_inlines(&parse_inline_spans(text), color_fg_bright())
}

/// Parse inline markdown with a custom base foreground color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    apply_color_to_inlines(&parse_inline_spans(text), base_color)
}

/// Convert `MdSpan` slices to ratatui `Span`s.
pub fn md_to_spans(md_spans: &[MdSpan]) -> Vec<Span<'static>> {
    md_spans
        .iter()
        .map(|s| Span::styled(s.content.clone(), s.style))
        .collect()
}
