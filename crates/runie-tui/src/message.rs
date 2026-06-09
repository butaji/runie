//! Message rendering — timestamps, margins, alignment.

use ratatui::{
    style::{Style},
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

pub fn render_user_message(
    content: &str,
    timestamp: f64,
    content_width: u16,
) -> Vec<Line<'static>> {
    let ts_str = format_timestamp(timestamp);
    let base_style = style_user();
    let bg_color = darken(color_border(), 0.5);
    let bg_style = Style::default().bg(bg_color);

    let mut lines = Vec::new();
    let content_lines: Vec<&str> = content.lines().collect();
    for (i, line) in content_lines.iter().enumerate() {
        lines.push(build_user_content_line(
            line, i == 0, GLYPH_USER, &ts_str, content_width, base_style, bg_style,
        ));
    }

    if content_lines.is_empty() {
        lines.push(build_user_content_line(
            "", true, GLYPH_USER, &ts_str, content_width, base_style, bg_style,
        ));
    }

    lines
}

fn build_user_content_line(
    line: &str,
    is_first: bool,
    first_prefix: &'static str,
    ts_str: &str,
    content_width: u16,
    base_style: Style,
    bg_style: Style,
) -> Line<'static> {
    let prefix = if is_first { first_prefix } else { GLYPH_INDENT };
    let mut spans = vec![
        Span::styled(" ", bg_style),
        Span::styled(prefix, base_style),
    ];
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(line, color_fg_bright())));

    let inner_width = content_width.saturating_sub(1);

    if is_first && inner_width > 0 {
        let ts_width = ts_str.len() as u16 + 1;
        let prefix_width = prefix.chars().count() as u16;
        let line_text_width = prefix_width + line.chars().count() as u16;
        let padding = inner_width.saturating_sub(line_text_width).saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::styled(" ".repeat(padding as usize), base_style));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }

    // Fill remainder so background color covers full width
    let used: u16 = spans.iter().map(|s| s.content.chars().count() as u16).sum();
    let fill = inner_width.saturating_sub(used);
    if fill > 0 {
        spans.push(Span::styled(" ".repeat(fill as usize), bg_style));
    }

    Line::from(spans).style(bg_style)
}

pub fn render_agent_message(
    content: &str,
    timestamp: f64,
    provider: &str,
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
                    lines.push(render_msg_line(line, is_first, provider, inner_width, &ts_str));
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
                    lines.push(render_list_item(item, ordered, i, is_first, inner_width, &ts_str));
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
        lines.push(render_empty_agent_line(provider, inner_width, &ts_str));
    }
    add_lr_margins_to_lines(lines)
}

fn render_msg_line(
    line: &str,
    is_first: bool,
    provider: &str,
    content_width: u16,
    ts_str: &str,
) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    let mut spans = vec![Span::styled(prefix.to_string(), style_agent())];
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(line, color_fg())));
    if is_first && !provider.is_empty() {
        spans.push(Span::styled(format!(" · {}", provider), style_timestamp()));
    }
    if is_first && content_width > 0 {
        let text_len: u16 = spans.iter().map(|s| s.content.chars().count() as u16).sum();
        let ts_width = ts_str.len() as u16 + 1;
        let padding = content_width.saturating_sub(text_len).saturating_sub(ts_width);
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }
    Line::from(spans)
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

fn render_empty_agent_line(provider: &str, content_width: u16, ts_str: &str) -> Line<'static> {
    let mut text = GLYPH_AGENT.to_string();
    if !provider.is_empty() {
        text.push_str(&format!(" {}", provider));
    }
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
    let lines: Vec<Line<'static>> = content.lines()
        .map(|line| Line::from(line.to_string()).style(style_thought()))
        .collect();
    add_lr_margins_to_lines(lines)
}

pub fn render_thinking(started: std::time::Instant) -> Vec<Line<'static>> {
    let lines = vec![Line::from(
        crate::theme::thinking_line(started.elapsed().as_secs_f64())
    ).style(style_thinking())];
    add_lr_margins_to_lines(lines)
}

pub fn render_thought_summary(content: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let first_line = content.lines().next().unwrap_or(content);
    let lines = vec![Line::from(
        format!("{} [+]", first_line)
    ).style(style_thought())];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_running(name: &str, duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![Line::from(
        format!("{} Running {}... {:.1}s", "⠋", name, duration_secs)
    ).style(style_tool_running())];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_done(name: &str, duration_secs: f64, output: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(tool_done_header(name, duration_secs))
        .style(style_tool_header())];
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
    let lines = vec![Line::from(
        format!("✓ {} {:.1}s [+]", name, duration_secs)
    ).style(style_tool_summary())];
    add_lr_margins_to_lines(lines)
}

pub fn render_turn_complete(duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![Line::from(
        format!("Turn completed in {:.1}s", duration_secs)
    ).style(style_turn_complete())];
    add_lr_margins_to_lines(lines)
}
