//! Code block rendering.

use ratatui::text::{Line, Span};

use crate::syntax::highlight_code;
use crate::theme::{code_header_label, style_code_header, GLYPH_INDENT};

pub(super) fn render_code_header(lang: &str) -> Line<'static> {
    let label = code_header_label("", lang);
    Line::from(Span::styled(label, style_code_header())).style(style_code_header())
}

pub(super) fn render_code_block_lines(content: &str, lang: &str) -> Vec<Line<'static>> {
    let highlighted = highlight_code(content, lang);
    highlighted
        .into_iter()
        .map(|tokens| {
            let mut spans = vec![Span::raw(GLYPH_INDENT.to_string())];
            for token in tokens {
                spans.push(Span::styled(token.content, token.style));
            }
            Line::from(spans)
        })
        .collect()
}
