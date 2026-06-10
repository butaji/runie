//! Message rendering — timestamps, margins, alignment.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use runie_core::format_timestamp;

use crate::markdown::{extract_code_blocks, md_to_spans, parse_inline_markdown_with_color, CodeBlock};
use crate::syntax::highlight_code;
use crate::theme::{
    GLYPH_USER, GLYPH_AGENT, GLYPH_INDENT,
    code_header_label,
    style_user, style_agent, style_thought, style_code_header,
    style_tool_header, style_tool_output, style_tool_running, style_tool_summary,
    style_turn_complete, style_thinking, style_timestamp,
    color_fg, color_fg_bright, color_border, darken,
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
    spans.iter().map(|s| s.content.chars().count() as u16).sum()
}

fn margin_line(width: u16, style: Style) -> Line<'static> {
    Line::from(" ".repeat(width as usize)).style(style)
}

fn push_flush(result: &mut Vec<String>, current: &mut String, width: &mut u16, max: u16) {
    if !current.is_empty() {
        result.push(std::mem::take(current));
        *width = 0;
    }
}

fn force_split_word(word: &str, max: u16, result: &mut Vec<String>, current: &mut String, width: &mut u16, rest_width: u16) {
    let mut chars = word.chars().peekable();
    while chars.peek().is_some() {
        if *width >= max {
            push_flush(result, current, width, max);
        }
        current.push(chars.next().unwrap());
        *width += 1;
    }
}

/// Word-wrap text so the first line reserves `first_width` chars and
/// subsequent lines use `rest_width`.  Never drops words.
fn word_wrap(text: &str, first_width: u16, rest_width: u16) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut width = 0u16;
    let mut max = first_width.max(1);

    for word in text.split_whitespace() {
        let w = word.chars().count() as u16;
        let need_space = !current.is_empty();

        if need_space && width + 1 + w > max {
            push_flush(&mut result, &mut current, &mut width, max);
            max = rest_width.max(1);
        }

        if !need_space && w > max {
            force_split_word(word, max, &mut result, &mut current, &mut width, rest_width.max(1));
            continue;
        }

        if need_space {
            current.push(' ');
            width += 1;
        }
        current.push_str(word);
        width += w;
    }

    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() && text.is_empty() {
        result.push(String::new());
    }
    result
}

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
    let p_width = prefix.chars().count() as u16;
    let mut spans = vec![
        Span::styled(" ", bg_style),
        Span::styled(prefix, base_style),
    ];
    spans.extend(md_to_spans(
        &parse_inline_markdown_with_color(chunk, color_fg_bright()),
    ));

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
    let padding = inner_width.saturating_sub(prefix_width).saturating_sub(ts_width);
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
    let bg_style = Style::default().bg(darken(color_border(), 0.5));
    let inner_width = content_width.saturating_sub(2);
    let prefix_width = GLYPH_USER.chars().count() as u16;
    let indent_width = GLYPH_INDENT.chars().count() as u16;
    let ts_width = ts_str.len() as u16 + 1;

    let mut lines = Vec::new();
    lines.push(margin_line(content_width, bg_style));

    let explicit_lines: Vec<&str> = content.lines().collect();
    let mut is_first = true;

    for explicit_line in explicit_lines.iter() {
        let first_w = if is_first {
            inner_width.saturating_sub(prefix_width).saturating_sub(ts_width)
        } else {
            inner_width.saturating_sub(indent_width)
        };
        let rest_w = inner_width.saturating_sub(indent_width);

        let wrapped = word_wrap(explicit_line, first_w, rest_w);
        for (j, chunk) in wrapped.iter().enumerate() {
            let prefix = if is_first && j == 0 { GLYPH_USER } else { GLYPH_INDENT };
            let with_ts = is_first && j == 0;
            lines.push(build_user_line(
                chunk, prefix, inner_width, &ts_str, ts_width, base_style, bg_style, with_ts,
            ));
        }
        is_first = false;
    }

    if lines.len() == 1 {
        lines.push(empty_user_line(inner_width, prefix_width, &ts_str, ts_width, base_style, bg_style));
    }

    lines.push(margin_line(content_width, bg_style));
    lines
}

pub fn render_agent_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let blocks = extract_code_blocks(content);
    let mut lines = Vec::new();
    let mut is_first = true;
    let ts_str = format_timestamp(timestamp);
    let inner_width = content_width.saturating_sub(2);

    for block in blocks {
        match block {
            CodeBlock::Text(text) => {
                for line in text.lines() {
                    lines.extend(render_msg_line(
                        line, is_first, inner_width, &ts_str,
                    ));
                    is_first = false;
                }
            }
            CodeBlock::Code { lang, content } => {
                lines.push(render_code_header(&lang, is_first, inner_width, &ts_str));
                is_first = false;
                lines.extend(render_code_block_lines(&content, &lang));
            }
            CodeBlock::List { ordered, items } => {
                for (i, item) in items.iter().enumerate() {
                    lines.push(render_list_item(
                        item, ordered, i, is_first, inner_width, &ts_str,
                    ));
                    is_first = false;
                }
            }
            CodeBlock::Blockquote(text) => {
                lines.extend(render_blockquote_lines(&text));
                is_first = false;
            }
        }
    }

    if lines.is_empty() {
        lines.push(render_empty_agent_line(inner_width, &ts_str));
    }
    add_lr_margins_to_lines(lines)
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
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(chunk, color_fg())));

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
    let prefix_width = GLYPH_AGENT.chars().count() as u16;
    let ts_width = if is_first { ts_str.len() as u16 + 1 } else { 0 };

    let first_w = if is_first {
        content_width.saturating_sub(prefix_width).saturating_sub(ts_width)
    } else {
        content_width.saturating_sub(prefix_width)
    };
    let rest_w = content_width.saturating_sub(prefix_width);

    let wrapped = word_wrap(line, first_w, rest_w);
    let mut lines = Vec::new();
    let mut first_done = false;

    for chunk in wrapped.iter() {
        let p = if is_first && !first_done { GLYPH_AGENT } else { GLYPH_INDENT };
        lines.push(build_agent_line(
            chunk, p, prefix_width, content_width, ts_str, ts_width,
            is_first && !first_done,
        ));
        first_done = true;
    }

    lines
}

fn render_code_header(lang: &str, is_first: bool, content_width: u16, ts_str: &str) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    let label = code_header_label(prefix, lang);
    let mut spans = vec![Span::styled(label.clone(), style_code_header())];
    if is_first && content_width > 0 {
        let text_len = label.chars().count() as u16;
        let ts_width = ts_str.len() as u16 + 1;
        let padding = content_width.saturating_sub(text_len).saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans).style(style_code_header())
}

fn render_code_block_lines(content: &str, lang: &str) -> Vec<Line<'static>> {
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

fn render_list_item(
    item: &str,
    ordered: bool,
    idx: usize,
    is_first: bool,
    content_width: u16,
    ts_str: &str,
) -> Line<'static> {
    let bullet = if ordered { format!("{}.", idx + 1) } else { "•".to_string() };
    let first_line_prefix = if is_first {
        format!("{} {}", GLYPH_AGENT, bullet)
    } else {
        format!("{} {}", GLYPH_INDENT, bullet)
    };
    let rest_prefix = format!("{}   ", GLYPH_INDENT);
    let lines: Vec<&str> = item.lines().collect();
    let mut result_spans: Vec<Span<'static>> = Vec::new();
    let mut text_len = 0u16;

    for (j, line) in lines.iter().enumerate() {
        let prefix = if j == 0 { &first_line_prefix } else { &rest_prefix };
        if j > 0 {
            result_spans.push(Span::raw("\n".to_string()));
        }
        result_spans.push(Span::styled(prefix.clone(), style_agent()));
        result_spans.push(Span::styled(line.to_string(), style_agent()));
        text_len = prefix.chars().count() as u16 + line.chars().count() as u16;
    }

    if is_first && content_width > 0 {
        let ts_width = ts_str.len() as u16 + 1;
        let padding = content_width.saturating_sub(text_len).saturating_sub(ts_width);
        if padding > 0 {
            result_spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        result_spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }

    Line::from(result_spans).style(style_agent())
}

fn render_blockquote_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|line| Line::from(format!("{}│ {}", GLYPH_INDENT, line)).style(style_agent()))
        .collect()
}

fn render_empty_agent_line(content_width: u16, ts_str: &str) -> Line<'static> {
    let text = GLYPH_AGENT.to_string();
    let mut spans = vec![Span::styled(text.clone(), style_agent())];
    if content_width > 0 {
        let ts_width = ts_str.len() as u16 + 1;
        let padding = content_width
            .saturating_sub(text.chars().count() as u16)
            .saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans).style(style_agent())
}

pub fn render_thought_marker(content: &str) -> Vec<Line<'static>> {
    let lines: Vec<Line<'static>> = content
        .lines()
        .map(|line| Line::from(line.to_string()).style(style_thought()))
        .collect();
    add_lr_margins_to_lines(lines)
}

pub fn render_thinking(started: std::time::Instant) -> Vec<Line<'static>> {
    let lines = vec![Line::from(
        crate::theme::thinking_line(started.elapsed().as_secs_f64()),
    )
    .style(style_thinking())];
    add_lr_margins_to_lines(lines)
}

pub fn render_thought_summary(content: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let first_line = content.lines().next().unwrap_or(content);
    let lines = vec![Line::from(format!("{} [+]", first_line)).style(style_thought())];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_running(name: &str, duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![Line::from(format!(
        "{} Running {}... {:.1}s",
        "⠋",
        name,
        duration_secs
    ))
    .style(style_tool_running())];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_done(name: &str, duration_secs: f64, output: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(tool_done_header(name, duration_secs)).style(style_tool_header())];
    if !output.is_empty() {
        if crate::diff::is_diff_output(output) {
            lines.extend(crate::diff::render_diff_text(output));
        } else {
            for line in output.lines() {
                lines.push(Line::from(line.to_string()).style(style_tool_output()));
            }
        }
    }
    add_lr_margins_to_lines(lines)
}

fn tool_done_header(name: &str, duration_secs: f64) -> String {
    format!("✓ {} {:.1}s", name, duration_secs)
}

pub fn render_tool_summary(name: &str, duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![Line::from(format!(
        "✓ {} {:.1}s [+]",
        name,
        duration_secs
    ))
    .style(style_tool_summary())];
    add_lr_margins_to_lines(lines)
}

pub fn render_turn_complete(duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![Line::from(format!(
        "Turn completed in {:.1}s",
        duration_secs
    ))
    .style(style_turn_complete())];
    add_lr_margins_to_lines(lines)
}
