//! Message rendering — timestamps, margins, alignment.
//!
//! Uses the core markdown module for block structure, with tui-markdown
//! providing inline styling for text blocks.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use unicode_width::UnicodeWidthStr;

/// Display-cell width for any `AsRef<str>` type.
fn str_width(s: impl AsRef<str>) -> u16 {
    UnicodeWidthStr::width(s.as_ref()) as u16
}
use runie_core::labels::format_timestamp;
use runie_core::markdown::{extract_code_blocks, inlines_to_text, CodeBlock};

use crate::markdown_render::{apply_color_to_inlines, md_to_spans, MdSpan};
use crate::theme::{
    color_agent_text, color_user_text, style_agent, style_feed_timestamp, style_user, GLYPH_INDENT,
    GLYPH_USER,
};

mod code;
mod support;
mod wrap;

pub(crate) use wrap::word_wrap;
use wrap::wrap_styled_spans;

pub use support::{
    render_context_group, render_subagent_row, render_thinking, render_thought_marker,
    render_thought_summary, render_tool_done, render_tool_running, render_tool_summary,
    render_turn_complete,
};

fn span_width(spans: &[Span<'_>]) -> u16 {
    spans.iter().map(|s| str_width(&s.content)).sum()
}



pub fn render_user_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let ts_str = format_timestamp(timestamp);
    let base_style = style_user();
    let inner_width = content_width;
    let prefix_width = str_width(GLYPH_USER);
    let indent_width = str_width(GLYPH_INDENT);
    let ts_width = str_width(&ts_str) + 1;

    // Reserve space for timestamp on first line to prevent wrapping/overflow
    let first_w = inner_width
        .saturating_sub(prefix_width)
        .saturating_sub(ts_width)
        .max(10); // Ensure at least 10 chars for content
    let rest_w = inner_width.saturating_sub(indent_width);

    // The user "card" is content plus one blank padding line above and below.
    // Those blank rows render with the bg.user background (applied in the feed
    // renderer to every UserMessage row), forming the card's top/bottom margin.
    let mut lines = Vec::with_capacity(4);
    lines.push(Line::from(""));
    lines.extend(build_user_body(
        content,
        first_w,
        rest_w,
        &UserLineParams {
            inner_width,
            ts_str,
            ts_width,
            base_style,
        },
    ));
    lines.push(Line::from(""));
    lines
}

/// Parameters for building user message lines.
struct UserLineParams {
    inner_width: u16,
    ts_str: String,
    ts_width: u16,
    base_style: Style,
}

fn build_user_body(
    content: &str,
    first_w: u16,
    rest_w: u16,
    params: &UserLineParams,
) -> Vec<Line<'static>> {
    // Use tui-markdown for inline styling (applies inline styles + base color).
    // tui_markdown drops inline HTML entirely, so "<think>"-like user text
    // would vanish; escape '<' to keep user content verbatim (pulldown
    // resolves the entity back to '<' in text events).
    let escaped = content.replace('<', "&lt;");
    let spans = apply_color_to_inlines(&escaped, color_user_text());
    let rows = wrap_styled_spans(&spans, first_w, rest_w);

    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let with_ts = i == 0;
            let prefix = if with_ts { GLYPH_USER } else { GLYPH_INDENT };
            build_user_line_from_spans(row, prefix, with_ts, params)
        })
        .collect()
}

fn build_user_line_from_spans(
    spans: &[MdSpan],
    prefix: &'static str,
    with_ts: bool,
    params: &UserLineParams,
) -> Line<'static> {
    let p_width = str_width(prefix);
    let mut line_spans = vec![Span::styled(prefix, params.base_style)];
    line_spans.extend(md_to_spans(spans));

    if with_ts {
        let text_width = span_width(&line_spans[1..]);
        let padding = params
            .inner_width
            .saturating_sub(p_width)
            .saturating_sub(text_width)
            .saturating_sub(params.ts_width);
        if padding > 0 {
            line_spans.push(Span::styled(
                " ".repeat(padding as usize),
                params.base_style,
            ));
        }
        line_spans.push(Span::styled(
            format!(" {}", params.ts_str),
            style_feed_timestamp(),
        ));
    }

    Line::from(line_spans)
}

pub fn render_agent_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let blocks = extract_code_blocks(content);
    let ts_str = format_timestamp(timestamp);
    let ts_width = str_width(&ts_str) + 1;
    let inner_width = content_width;

    // Plain answer lines carry no leading glyph (grok parity): text starts
    // at the feed indent on every line, so wrapping uses the full width.
    // Reserve space for the timestamp on the first line only.
    let first_w = inner_width.saturating_sub(ts_width).max(10);
    let rest_w = inner_width;

    let mut lines = build_agent_body(&blocks, &ts_str, inner_width, first_w, rest_w);

    if lines.is_empty() {
        lines.push(render_empty_agent_line(inner_width, &ts_str));
    }
    lines
}

fn build_agent_body(blocks: &[CodeBlock], ts_str: &str, inner_width: u16, first_w: u16, rest_w: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut is_first = true;

    for block in blocks {
        is_first = render_agent_block(block, ts_str, inner_width, first_w, rest_w, is_first, &mut lines);
    }
    lines
}

fn render_agent_block(
    block: &CodeBlock,
    ts_str: &str,
    inner_width: u16,
    first_w: u16,
    rest_w: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    match block {
        CodeBlock::Text { inlines, .. } => {
            render_agent_text_block(inlines, ts_str, inner_width, first_w, rest_w, is_first, lines)
        }
        CodeBlock::Code { lang, content } => {
            render_agent_code_block(lang, content, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::List { ordered, items } => {
            render_agent_list_block(items, *ordered, ts_str, inner_width, first_w, rest_w, is_first, lines)
        }
        CodeBlock::Blockquote(inlines) => {
            let text = inlines_to_text(inlines);
            lines.extend(support::render_blockquote_from_spans(&text, color_agent_text()));
            false
        }
    }
}

fn render_agent_text_block(
    inlines: &[runie_core::markdown::MdInline],
    ts_str: &str,
    inner_width: u16,
    first_w: u16,
    rest_w: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    if inlines.is_empty() {
        return is_first;
    }
    // Convert MdInline[] to plain text, then style with tui_markdown.
    let text = inlines_to_text(inlines);
    let spans = apply_color_to_inlines(&text, color_agent_text());
    let ts_width = str_width(ts_str) + 1;
    let rows = wrap_styled_spans(&spans, first_w, rest_w);

    for (i, row) in rows.iter().enumerate() {
        let with_ts = is_first && i == 0;
        lines.push(build_agent_line_from_spans(
            row,
            inner_width,
            ts_str,
            ts_width,
            with_ts,
        ));
    }
    false
}

fn build_agent_line_from_spans(
    spans: &[MdSpan],
    content_width: u16,
    ts_str: &str,
    ts_width: u16,
    with_ts: bool,
) -> Line<'static> {
    let mut line_spans = md_to_spans(spans);

    if with_ts && content_width > 0 {
        let text_width = span_width(&line_spans);
        let padding = content_width
            .saturating_sub(text_width)
            .saturating_sub(ts_width);
        if padding > 0 {
            line_spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        line_spans.push(Span::styled(
            format!(" {}", ts_str),
            style_feed_timestamp(),
        ));
    }
    Line::from(line_spans)
}

fn render_agent_code_block(
    lang: &str,
    content: &str,
    ts_str: &str,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    lines.push(code::render_code_header(
        lang,
        is_first,
        inner_width,
        ts_str,
    ));
    lines.extend(code::render_code_block_lines(content, lang));
    false
}

fn render_agent_list_block(
    items: &[Vec<runie_core::markdown::MdInline>],
    ordered: bool,
    ts_str: &str,
    inner_width: u16,
    first_w: u16,
    rest_w: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    let mut first_item = is_first;
    for (i, item) in items.iter().enumerate() {
        if item.is_empty() {
            continue;
        }
        // Convert MdInline[] to plain text, then style with tui_markdown.
        let item_text = inlines_to_text(item);
        let spans = apply_color_to_inlines(&item_text, color_agent_text());
        let ts_width = str_width(ts_str) + 1;
        let rows = wrap_styled_spans(&spans, first_w, rest_w);

        for (j, row) in rows.iter().enumerate() {
            let with_ts = first_item && j == 0;
            lines.push(support::render_list_item_from_spans(
                row, ordered, i, with_ts, "", ts_str, ts_width, inner_width,
            ));
        }
        first_item = false;
    }
    is_first
}

fn render_empty_agent_line(content_width: u16, ts_str: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if content_width > 0 {
        let ts_width = str_width(ts_str) + 1;
        let padding = content_width.saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(
            format!(" {}", ts_str),
            style_feed_timestamp(),
        ));
    }
    Line::from(spans).style(style_agent())
}
