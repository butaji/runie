//! Message rendering — Grok-style feed: right-aligned user bubbles,
//! left-aligned plain agent text, no feed message prefixes.

use ratatui::text::{Line, Span};

use runie_core::display_width;

use crate::markdown::{
    apply_color_to_inlines, extract_code_blocks, md_to_spans, parse_inline_spans, CodeBlock,
    MdInline, MdSpan,
};
use crate::theme::{color_fg, color_fg_bright, color_user_bg, style_agent};

mod bubble;
mod code;
mod support;
mod wrap;

pub(crate) use wrap::word_wrap;
use wrap::wrap_styled_spans;

pub use support::{
    render_context_group, render_thinking, render_thought_marker, render_thought_summary,
    render_tool_done, render_tool_running, render_tool_summary, render_turn_complete,
};

const MARGIN_SYMBOL: &str = " ";
const BUBBLE_H_PAD: u16 = 2;

fn add_lr_margins(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw(MARGIN_SYMBOL.to_string())];
    spans.extend(line.spans.iter().cloned());
    spans.push(Span::raw(MARGIN_SYMBOL.to_string()));
    Line::from(spans).style(line.style)
}

fn add_lr_margins_to_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    lines.into_iter().map(add_lr_margins).collect()
}

fn span_width(spans: &[Span<'_>]) -> u16 {
    spans.iter().map(|s| display_width::width(&s.content)).sum()
}

pub fn render_user_message(
    content: &str,
    _timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let inner_width = content_width.saturating_sub(2);
    if inner_width == 0 {
        return vec![Line::from("")];
    }

    let text_width = inner_width.saturating_sub(BUBBLE_H_PAD * 2);
    let inlines = parse_inline_spans(content);
    let spans = apply_color_to_inlines(&inlines, color_fg_bright());
    let rows = wrap_styled_spans(&spans, text_width, text_width);

    let bubble_width = bubble::compute_width(&rows, inner_width);
    let left_fill = content_width.saturating_sub(bubble_width).saturating_sub(1);
    let bg = color_user_bg();

    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(bubble::margin_line(left_fill, bubble_width, content_width, bg));
    for row in rows {
        lines.push(bubble::content_line(
            &row,
            left_fill,
            bubble_width,
            BUBBLE_H_PAD,
            bg,
        ));
    }
    lines.push(bubble::margin_line(left_fill, bubble_width, content_width, bg));
    lines
}

pub fn render_agent_message(
    content: &str,
    _timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let blocks = extract_code_blocks(content);
    let inner_width = content_width.saturating_sub(2);
    let mut lines = build_agent_body(&blocks, inner_width);

    if lines.is_empty() {
        lines.push(Line::from("").style(style_agent()));
    }
    add_lr_margins_to_lines(lines)
}

fn build_agent_body(blocks: &[CodeBlock], inner_width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut is_first = true;

    for block in blocks {
        is_first = render_agent_block(block, inner_width, is_first, &mut lines);
    }
    lines
}

fn render_agent_block(
    block: &CodeBlock,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    match block {
        CodeBlock::Text { inlines, .. } => render_agent_text_block(inlines, inner_width, lines),
        CodeBlock::Code { lang, content } => {
            render_agent_code_block(lang, content, inner_width, is_first, lines)
        }
        CodeBlock::List { ordered, items } => {
            render_agent_list_block(items, *ordered, inner_width, lines)
        }
        CodeBlock::Blockquote(text) => {
            lines.extend(support::render_blockquote_lines(text));
            false
        }
    }
}

fn render_agent_text_block(
    inlines: &[MdInline],
    inner_width: u16,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    if inlines.is_empty() {
        return false;
    }
    let spans = apply_color_to_inlines(inlines, color_fg());
    let rows = wrap_styled_spans(&spans, inner_width, inner_width);

    for row in rows {
        lines.push(build_agent_line_from_spans(&row));
    }
    false
}

fn render_agent_code_block(
    lang: &str,
    content: &str,
    _inner_width: u16,
    _is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    lines.push(code::render_code_header(lang));
    lines.extend(code::render_code_block_lines(content, lang));
    false
}

fn render_agent_list_block(
    items: &[String],
    ordered: bool,
    inner_width: u16,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    for (i, item) in items.iter().enumerate() {
        lines.push(support::render_list_item(
            item, ordered, i, false, inner_width, "",
        ));
    }
    false
}

fn build_agent_line_from_spans(spans: &[MdSpan]) -> Line<'static> {
    Line::from(md_to_spans(spans)).style(style_agent())
}
