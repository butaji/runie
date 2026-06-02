use ratatui::{buffer::Buffer, layout::Rect, style::{Modifier, Style}, text::{Line, Span}, widgets::Widget};

use crate::components::message_list::WrapCache;
use crate::components::message_list::feed::ToolCall;
use crate::glyphs;
use crate::messages::MessageRegistry;
use super::markdown::render_text_content;
use super::tool::render_tool_call_inline;

/// Extract think blocks from text and returns (main_text, think_blocks).
/// DeepSeek models use these for internal reasoning.
pub fn extract_think_blocks(text: &str) -> (String, Vec<String>) {
    // Fast path: no think tags → zero allocation
    if !text.contains("<think>") {
        return (text.to_string(), Vec::new());
    }

    let bytes = text.as_bytes();
    let mut main_text = String::new();
    let mut think_blocks = Vec::new();
    let mut i = 0;
    let mut last_end = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(b"<think>") {
            let start = i;
            if let Some((block_start, block_end, new_i)) = find_closing_tag(bytes, i + 7) {
                i = new_i;
                main_text.push_str(&text[last_end..start]);
                push_think_block(text, block_start, block_end, &mut think_blocks);
                last_end = i;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }

    if last_end < text.len() {
        main_text.push_str(&text[last_end..]);
    }

    (main_text, think_blocks)
}

fn find_closing_tag(bytes: &[u8], start: usize) -> Option<(usize, usize, usize)> {
    let mut j = start;
    while j < bytes.len() {
        if bytes[j..].starts_with(b"</think>") {
            return Some((start, j, j + 8));
        }
        j += 1;
    }
    None
}

fn push_think_block(text: &str, block_start: usize, block_end: usize, think_blocks: &mut Vec<String>) {
    let think_content = text[block_start..block_end].trim();
    if !think_content.is_empty() {
        think_blocks.push(think_content.to_string());
    }
}

/// Strips think blocks from text (DeepSeek models use these).
pub fn strip_think_tags(text: &str) -> String {
    extract_think_blocks(text).0
}

fn render_think_block_box(
    think_content: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_muted: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
    buf: &mut Buffer,
) -> u16 {
    let inner_width = (area.width - margin_x + area.x - 6) as usize;
    let wrapped = wrap_cache.get_wrapped(think_content, inner_width);
    let mut rendered = 0u16;

    for line_text in wrapped {
        let line_y = row + rendered;
        if line_y >= area.height {
            break;
        }
        let content = format!("{} {}", glyphs::DOT, line_text);
        let line = ratatui::text::Line::raw(content).style(Style::default().fg(text_muted));
        buf.set_line(margin_x, area.y + line_y, &line, area.width - margin_x + area.x - 2);
        rendered += 1;
    }

    rendered
}

/// Render an assistant message
pub fn render_assistant_msg(
    text: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    _accent_bar_color: ratatui::style::Color,
    cursor_visible: bool,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
    spinner: char,
    timestamp: Option<&str>,
    thought_duration: Option<f32>,
    turn_complete: Option<u64>,
    _is_last_item: bool,
    tool_calls: &[ToolCall],
    tool_bar_color: ratatui::style::Color,
    thoughts_collapsed: bool,
) -> u16 {
    let (stripped, _think_blocks) = extract_think_blocks(text);

    if stripped.trim().is_empty() && _think_blocks.is_empty() {
        // Never show just a dot — always show meaningful status
        let content = if agent_running {
            format!("{} {}...", spinner, MessageRegistry::status_thinking())
        } else {
            format!("{} {}", glyphs::DOT, MessageRegistry::status_waiting())
        };
        let para = ratatui::widgets::Paragraph::new(ratatui::text::Line::raw(content).style(Style::default().fg(text_muted)))
            .style(Style::default().fg(text_muted));
        let para_area = Rect::new(margin_x, area.y + row, area.width - margin_x + area.x - 2, 1);
        para.render(para_area, buf);
        return 1;
    }

    let width = (area.width - margin_x + area.x - 2) as usize;
    let content_width = area.width - margin_x + area.x - 2;
    let mut rendered = 0u16;

    // When thoughts_collapsed is true, render collapsed indicator instead of full content
    if thoughts_collapsed {
        if let Some(duration) = thought_duration {
            // Draw vertical bar at left edge
            if let Some(cell) = buf.cell_mut((margin_x.saturating_sub(1), area.y + row)) {
                cell.set_char('┃');
                cell.set_style(Style::default().fg(text_muted));
            }
            let header_text = format!("{} ◆ Thought for {:.1}s ▶", glyphs::CHEVRON, duration);
            let header = Line::styled(header_text, Style::default().fg(text_muted).add_modifier(Modifier::BOLD));
            buf.set_line(margin_x, area.y + row, &header, content_width);
            return 1;
        }
    }

    // Render thought indicator BEFORE answer (if thought_duration is provided)
    // ◆ {Thought for Xs} - diamond muted, entire phrase from MessageRegistry
    let thought_rows = if let Some(duration) = thought_duration {
        let duration_text = MessageRegistry::thought_duration(duration);
        let line = Line::from(vec![
            Span::raw(format!("{} ", glyphs::THOUGHT_MARKER)).style(Style::default().fg(text_muted)),
            Span::raw(&duration_text).style(Style::default().fg(text_muted)),
        ]);
        buf.set_line(margin_x, area.y + row, &line, content_width);
        1
    } else {
        0
    };
    rendered += thought_rows;

    // Render tool calls inline (between thought indicator and markdown content)
    let tool_call_start_row = rendered;
    for tool_call in tool_calls {
        if row + rendered >= max_rows {
            break;
        }
        let rows = render_tool_call_inline(
            &tool_call.name,
            &tool_call.args,
            area,
            row + rendered,
            margin_x,
            content_width,
            buf,
            tool_bar_color,
            text_muted,
        );
        rendered += rows;
    }
    // Render think blocks if any (not currently used but preserved)
    for think in &_think_blocks {
        if row + rendered >= max_rows {
            break;
        }
        let block_rows = render_think_block_box(think, area, row + rendered, margin_x, text_muted, wrap_cache, buf);
        rendered += block_rows;
    }

    if stripped.trim().is_empty() {
        return rendered;
    }

    let base_style = Style::default().add_modifier(Modifier::BOLD).fg(text_secondary);
    let stripped_trimmed = stripped.trim_start();
    let mut markdown_lines = render_text_content(stripped_trimmed, width, base_style);

    // Prepend ∘ bullet to first line of assistant response
    if !markdown_lines.is_empty() {
        // 3 leading spaces + bullet + space = 5 chars (margin_x + 2 + 5 = area.x + 5 = 5 spaces from edge)
        let bullet_span = Span::raw(format!("   {} ", glyphs::ASSISTANT_BULLET)).style(base_style);
        let first_line = &mut markdown_lines[0];
        let mut new_spans = vec![bullet_span];
        new_spans.append(&mut first_line.spans);
        first_line.spans = new_spans;
    }

    let text_rows = markdown_lines.len() as u16;

    for (i, line) in markdown_lines.iter().enumerate() {
        let line_y = row + rendered + i as u16;
        if line_y >= max_rows {
            break;
        }
        buf.set_line(margin_x, area.y + line_y, line, area.width - margin_x + area.x - 2);
    }
    rendered += text_rows;

    // Render right-aligned timestamp on the LAST LINE of text content (before "Turn completed")
    if let Some(ts) = timestamp {
        if rendered > 0 && row + rendered - 1 < max_rows {
            let ts_len = ts.len() as u16;
            let available_width = area.width - margin_x + area.x - 2;
            let ts_x = if ts_len < available_width {
                margin_x + available_width - ts_len
            } else {
                margin_x
            };
            let line = ratatui::text::Line::raw(ts).style(Style::default().fg(text_muted));
            buf.set_line(ts_x, area.y + row + rendered - 1, &line, ts_len);
        }
    }

    // Render "Turn completed in Xs" at bottom (only when turn is done, not streaming)
    if !agent_running {
        if let Some(elapsed) = turn_complete {
            if row + rendered < max_rows {
                let complete_text = MessageRegistry::turn_completed(elapsed as f32);
                let line = ratatui::text::Line::raw(complete_text).style(Style::default().fg(text_muted));
                buf.set_line(margin_x, area.y + row + rendered, &line, content_width);
                rendered += 1;
            }
        }
    }

    // Cursor placement at end of rendered content
    if cursor_visible && text_rows > 0 {
        let cursor_y = area.y + row + rendered - 1;
        let last_line_len = markdown_lines.last().map(|l| l.width()).unwrap_or(0);
        let cursor_x = margin_x + (last_line_len as u16).min(area.width - margin_x + area.x - 3);
        if cursor_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                cell.set_char(glyphs::CURSOR_BLOCK);
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
    rendered
}
