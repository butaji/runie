//! Code block rendering.

use ratatui::text::{Line, Span};

use crate::syntax::highlight_code;
use crate::theme::{
    code_header_label, style_code_header, style_feed_timestamp, GLYPH_AGENT, GLYPH_INDENT,
};
use unicode_width::UnicodeWidthStr;

pub(super) fn render_code_header(
    lang: &str,
    is_first: bool,
    content_width: u16,
    ts_str: &str,
) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    let label = code_header_label(prefix, lang);
    let mut spans = vec![Span::styled(label.clone(), style_code_header())];
    if is_first && content_width > 0 {
        let text_len = UnicodeWidthStr::width(label.as_str()) as u16;
        let ts_width = UnicodeWidthStr::width(ts_str) as u16 + 1;
        let padding = content_width
            .saturating_sub(text_len)
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_feed_timestamp()));
    }
    Line::from(spans).style(style_code_header())
}

pub(super) fn render_code_block_lines(content: &str, lang: &str) -> Vec<Line<'static>> {
    let highlighted = highlight_code(content, lang);
    highlighted
        .into_iter()
        .map(|tokens| {
            let mut spans = vec![Span::raw(GLYPH_INDENT.to_owned())];
            for token in tokens {
                spans.push(Span::styled(token.content, token.style));
            }
            Line::from(spans)
        })
        .collect()
}
