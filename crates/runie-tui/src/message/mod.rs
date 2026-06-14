//! Message rendering — timestamps, margins, alignment.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use runie_core::display_width;
use runie_core::format_timestamp;

use crate::markdown::{
    extract_code_blocks, md_to_spans, parse_inline_markdown_with_color, CodeBlock,
};
use crate::theme::{
    color_accent_bg, color_fg, color_fg_bright, style_agent, style_timestamp, style_user,
    GLYPH_AGENT, GLYPH_INDENT, GLYPH_USER,
};

mod code;
mod support;
mod wrap;

pub use support::{
    render_thinking, render_thought_marker, render_thought_summary, render_tool_done,
    render_tool_running, render_tool_summary, render_turn_complete,
};

const MARGIN_SYMBOL: &str = " ";

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

fn margin_line(width: u16, style: Style) -> Line<'static> {
    Line::from(" ".repeat(width as usize)).style(style)
}

use wrap::word_wrap;

#[allow(clippy::too_many_arguments)]
fn build_user_line(
    chunk: &str,
    prefix: &'static str,
    inner_width: u16,
    ts_str: &str,
    ts_width: u16,
    base_style: Style,
    bg_style: Style,
    with_ts: bool,
) -> Line<'static> {
    let p_width = display_width::width(prefix);
    let mut spans = vec![
        Span::styled(" ", bg_style),
        Span::styled(prefix, base_style),
    ];
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(
        chunk,
        color_fg_bright(),
    )));

    if with_ts {
        let text_width = span_width(&spans[2..]);
        let padding = inner_width
            .saturating_sub(p_width)
            .saturating_sub(text_width)
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::styled(" ".repeat(padding as usize), base_style));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }

    let used = span_width(&spans);
    let fill = inner_width.saturating_sub(used);
    if fill > 0 {
        spans.push(Span::styled(" ".repeat(fill as usize), bg_style));
    }
    spans.push(Span::styled(" ", bg_style));
    Line::from(spans).style(bg_style)
}

fn empty_user_line(
    inner_width: u16,
    prefix_width: u16,
    ts_str: &str,
    ts_width: u16,
    base_style: Style,
    bg_style: Style,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(" ", bg_style),
        Span::styled(GLYPH_USER, base_style),
    ];
    let padding = inner_width
        .saturating_sub(prefix_width)
        .saturating_sub(ts_width);
    if padding > 0 {
        spans.push(Span::styled(" ".repeat(padding as usize), base_style));
    }
    spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    let used = span_width(&spans);
    let fill = inner_width.saturating_sub(used);
    if fill > 0 {
        spans.push(Span::styled(" ".repeat(fill as usize), bg_style));
    }
    spans.push(Span::styled(" ", bg_style));
    Line::from(spans).style(bg_style)
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
        inner_width,
        prefix_width,
        indent_width,
        ts_width,
        &ts_str,
        base_style,
        bg_style,
    ));

    if lines.len() == 1 {
        lines.push(empty_user_line(
            inner_width,
            prefix_width,
            &ts_str,
            ts_width,
            base_style,
            bg_style,
        ));
    }

    lines.push(margin_line(content_width, bg_style));
    lines
}

#[allow(clippy::too_many_arguments)]
fn build_user_body(
    content: &str,
    inner_width: u16,
    prefix_width: u16,
    indent_width: u16,
    ts_width: u16,
    ts_str: &str,
    base_style: Style,
    bg_style: Style,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let explicit_lines: Vec<&str> = content.lines().collect();
    let mut is_first = true;

    for explicit_line in explicit_lines.iter() {
        append_user_wrapped(
            explicit_line,
            &mut lines,
            &mut is_first,
            inner_width,
            prefix_width,
            indent_width,
            ts_width,
            ts_str,
            base_style,
            bg_style,
        );
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn append_user_wrapped(
    line: &str,
    lines: &mut Vec<Line<'static>>,
    is_first: &mut bool,
    inner_width: u16,
    prefix_width: u16,
    indent_width: u16,
    ts_width: u16,
    ts_str: &str,
    base_style: Style,
    bg_style: Style,
) {
    let rest_w = inner_width.saturating_sub(indent_width);
    let first_w = if *is_first {
        inner_width
            .saturating_sub(prefix_width)
            .saturating_sub(ts_width)
    } else {
        rest_w
    };
    for (j, chunk) in word_wrap(line, first_w, rest_w).iter().enumerate() {
        let (prefix, with_ts) = if *is_first && j == 0 {
            (GLYPH_USER, true)
        } else {
            (GLYPH_INDENT, false)
        };
        lines.push(build_user_line(
            chunk,
            prefix,
            inner_width,
            ts_str,
            ts_width,
            base_style,
            bg_style,
            with_ts,
        ));
    }
    *is_first = false;
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
        CodeBlock::Text(text) => {
            render_agent_text_block(text, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::Code { lang, content } => {
            render_agent_code_block(lang, content, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::List { ordered, items } => {
            render_agent_list_block(items, *ordered, ts_str, inner_width, is_first, lines)
        }
        CodeBlock::Blockquote(text) => {
            lines.extend(support::render_blockquote_lines(text));
            false
        }
    }
}

fn render_agent_text_block(
    text: &str,
    ts_str: &str,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    let mut is_first = is_first;
    for line in text.lines() {
        lines.extend(render_msg_line(line, is_first, inner_width, ts_str));
        is_first = false;
    }
    is_first
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
    items: &[String],
    ordered: bool,
    ts_str: &str,
    inner_width: u16,
    is_first: bool,
    lines: &mut Vec<Line<'static>>,
) -> bool {
    let mut is_first = is_first;
    for (i, item) in items.iter().enumerate() {
        lines.push(support::render_list_item(
            item,
            ordered,
            i,
            is_first,
            inner_width,
            ts_str,
        ));
        is_first = false;
    }
    is_first
}

fn build_agent_line(
    chunk: &str,
    prefix: &'static str,
    prefix_width: u16,
    content_width: u16,
    ts_str: &str,
    ts_width: u16,
    with_ts: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(prefix.to_string(), style_agent())];
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(
        chunk,
        color_fg(),
    )));

    if with_ts && content_width > 0 {
        let text_width = span_width(&spans[1..]);
        let padding = content_width
            .saturating_sub(prefix_width)
            .saturating_sub(text_width)
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans)
}

fn render_msg_line(
    line: &str,
    is_first: bool,
    content_width: u16,
    ts_str: &str,
) -> Vec<Line<'static>> {
    let prefix_width = display_width::width(GLYPH_AGENT);
    let ts_width = if is_first {
        display_width::width(ts_str) + 1
    } else {
        0
    };
    let (first_w, rest_w) = msg_line_widths(content_width, prefix_width, ts_width, is_first);

    let wrapped = word_wrap(line, first_w, rest_w);
    let mut lines = Vec::new();
    let mut first_done = false;

    for chunk in wrapped.iter() {
        lines.push(msg_chunk_line(
            chunk,
            is_first,
            &mut first_done,
            prefix_width,
            content_width,
            ts_str,
            ts_width,
        ));
    }

    lines
}

fn msg_line_widths(
    content_width: u16,
    prefix_width: u16,
    ts_width: u16,
    is_first: bool,
) -> (u16, u16) {
    let first_w = if is_first {
        content_width
            .saturating_sub(prefix_width)
            .saturating_sub(ts_width)
    } else {
        content_width.saturating_sub(prefix_width)
    };
    let rest_w = content_width.saturating_sub(prefix_width);
    (first_w, rest_w)
}

fn msg_chunk_line(
    chunk: &str,
    is_first: bool,
    first_done: &mut bool,
    prefix_width: u16,
    content_width: u16,
    ts_str: &str,
    ts_width: u16,
) -> Line<'static> {
    let p = if is_first && !*first_done {
        GLYPH_AGENT
    } else {
        GLYPH_INDENT
    };
    let line = build_agent_line(
        chunk,
        p,
        prefix_width,
        content_width,
        ts_str,
        ts_width,
        is_first && !*first_done,
    );
    *first_done = true;
    line
}

fn render_empty_agent_line(content_width: u16, ts_str: &str) -> Line<'static> {
    let text = GLYPH_AGENT.to_string();
    let mut spans = vec![Span::styled(text.clone(), style_agent())];
    if content_width > 0 {
        let ts_width = display_width::width(ts_str) + 1;
        let padding = content_width
            .saturating_sub(display_width::width(&text))
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans).style(style_agent())
}
