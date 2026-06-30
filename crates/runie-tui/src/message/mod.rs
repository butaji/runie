//! Message rendering — timestamps, margins, alignment.
//!
//! Uses the core markdown module for block structure, with tui-markdown
//! providing inline styling for text blocks.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use runie_util::display_width;
use runie_util::labels::format_timestamp;

use crate::markdown_render::{
    apply_color_to_inlines, extract_code_blocks, md_to_spans, parse_inline_spans, CodeBlock, MdSpan,
};
use crate::theme::{
    color_accent_bg, color_fg, color_fg_bright, style_agent, style_timestamp, style_user,
    GLYPH_AGENT, GLYPH_INDENT, GLYPH_USER,
};

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

fn add_lr_margins(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw(MARGIN_SYMBOL.to_owned())];
    spans.extend(line.spans.iter().cloned());
    spans.push(Span::raw(MARGIN_SYMBOL.to_owned()));
    Line::from(spans).style(line.style)
}

fn add_lr_margins_to_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    lines.into_iter().map(add_lr_margins).collect()
}

fn span_width(spans: &[Span<'_>]) -> u16 {
    spans.iter().map(|s| display_width::width(&s.content)).sum()
}

fn margin_line(width: u16, style: Style) -> Line<'static> {
    Line::from(" ".repeat(width as usize)).style(style)
}

pub fn render_user_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let ts_str = format_timestamp(timestamp);
    let base_style = style_user();
    let bg_style = Style::default().bg(color_accent_bg());
    let inner_width = content_width.saturating_sub(2);
    let prefix_width = display_width::width(GLYPH_USER);
    let indent_width = display_width::width(GLYPH_INDENT);
    let ts_width = display_width::width(&ts_str) + 1;

    let mut lines = Vec::new();
    lines.push(margin_line(content_width, bg_style));
    lines.extend(build_user_body(
        content,
        prefix_width,
        indent_width,
        &UserLineParams {
            inner_width,
            ts_str,
            ts_width,
            base_style,
            bg_style,
        },
    ));
    lines.push(margin_line(content_width, bg_style));
    lines
}

/// Parameters for building user message lines.
struct UserLineParams {
    inner_width: u16,
    ts_str: String,
    ts_width: u16,
    base_style: Style,
    bg_style: Style,
}

fn build_user_body(
    content: &str,
    prefix_width: u16,
    indent_width: u16,
    params: &UserLineParams,
) -> Vec<Line<'static>> {
    // Use tui-markdown for inline styling
    let inlines = parse_inline_spans(content);
    let spans = apply_color_to_inlines(&inlines, color_fg_bright());
    let first_w = params
        .inner_width
        .saturating_sub(prefix_width)
        .saturating_sub(params.ts_width);
    let rest_w = params.inner_width.saturating_sub(indent_width);
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
    let p_width = display_width::width(prefix);
    let mut line_spans = vec![
        Span::styled(" ", params.bg_style),
        Span::styled(prefix, params.base_style),
    ];
    line_spans.extend(md_to_spans(spans));

    if with_ts {
        let text_width = span_width(&line_spans[2..]);
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
            style_timestamp(),
        ));
    }

    let used = span_width(&line_spans);
    let fill = params.inner_width.saturating_sub(used);
    if fill > 0 {
        line_spans.push(Span::styled(" ".repeat(fill as usize), params.bg_style));
    }
    line_spans.push(Span::styled(" ", params.bg_style));
    Line::from(line_spans).style(params.bg_style)
}

pub fn render_agent_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let blocks = extract_code_blocks(content);
    let ts_str = format_timestamp(timestamp);
    let inner_width = content_width.saturating_sub(2);
    let mut lines = build_agent_body(&blocks, &ts_str, inner_width);

    if lines.is_empty() {
        lines.push(render_empty_agent_line(inner_width, &ts_str));
    }
    add_lr_margins_to_lines(lines)
}

fn build_agent_body(blocks: &[CodeBlock], ts_str: &str, inner_width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut is_first = true;

    for block in blocks {
        is_first = render_agent_block(block, ts_str, inner_width, is_first, &mut lines);
    }
    lines
}

fn render_agent_block(
    block: &CodeBlock,
    ts_str: &str,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    match block {
        CodeBlock::Text { inlines, .. } => {
            render_agent_text_block(inlines, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::Code { lang, content } => {
            render_agent_code_block(lang, content, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::List { ordered, items } => {
            render_agent_list_block(items, *ordered, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::Blockquote(inlines) => {
            lines.extend(support::render_blockquote_from_spans(inlines, color_fg()));
            false
        }
    }
}

fn render_agent_text_block(
    inlines: &[runie_core::markdown::MdInline],
    ts_str: &str,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    if inlines.is_empty() {
        return is_first;
    }
    let spans = apply_color_to_inlines(inlines, color_fg());
    let prefix_width = display_width::width(GLYPH_AGENT);
    let indent_width = display_width::width(GLYPH_INDENT);
    let ts_width = display_width::width(ts_str) + 1;
    let first_w = inner_width
        .saturating_sub(prefix_width)
        .saturating_sub(ts_width);
    let rest_w = inner_width.saturating_sub(indent_width);
    let rows = wrap_styled_spans(&spans, first_w, rest_w);

    for (i, row) in rows.iter().enumerate() {
        let with_ts = is_first && i == 0;
        let prefix = if with_ts { GLYPH_AGENT } else { GLYPH_INDENT };
        lines.push(build_agent_line_from_spans(
            row,
            prefix,
            prefix_width,
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
    prefix: &'static str,
    prefix_width: u16,
    content_width: u16,
    ts_str: &str,
    ts_width: u16,
    with_ts: bool,
) -> Line<'static> {
    let mut line_spans = vec![Span::styled(prefix.to_owned(), style_agent())];
    line_spans.extend(md_to_spans(spans));

    if with_ts && content_width > 0 {
        let text_width = span_width(&line_spans[1..]);
        let padding = content_width
            .saturating_sub(prefix_width)
            .saturating_sub(text_width)
            .saturating_sub(ts_width);
        if padding > 0 {
            line_spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        line_spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
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
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    let mut first_item = is_first;
    for (i, item) in items.iter().enumerate() {
        if item.is_empty() {
            continue;
        }
        let spans = apply_color_to_inlines(item, color_fg());
        let prefix_width = display_width::width(GLYPH_AGENT);
        let indent_width = display_width::width(GLYPH_INDENT);
        let ts_width = display_width::width(ts_str) + 1;
        let first_w = inner_width
            .saturating_sub(prefix_width)
            .saturating_sub(ts_width);
        let rest_w = inner_width.saturating_sub(indent_width);
        let rows = wrap_styled_spans(&spans, first_w, rest_w);

        for (j, row) in rows.iter().enumerate() {
            let with_ts = first_item && j == 0;
            let prefix = if with_ts { GLYPH_AGENT } else { GLYPH_INDENT };
            lines.push(support::render_list_item_from_spans(
                row,
                ordered,
                i,
                with_ts,
                prefix,
                ts_str,
                ts_width,
            ));
        }
        first_item = false;
    }
    is_first
}

fn render_empty_agent_line(content_width: u16, ts_str: &str) -> Line<'static> {
    let prefix = GLYPH_AGENT;
    let mut spans = vec![Span::styled(prefix, style_agent())];
    if content_width > 0 {
        let ts_width = display_width::width(ts_str) + 1;
        let padding = content_width
            .saturating_sub(display_width::width(prefix))
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans).style(style_agent())
}
