use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

use crate::components::message_list::WrapCache;
use super::markdown::render_text_content;

/// Extract think blocks from text and returns (main_text, think_blocks).
/// DeepSeek models use these for internal reasoning.
pub fn extract_think_blocks(text: &str) -> (String, Vec<String>) {
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
        let content = format!("· {}", line_text);
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
    cursor_visible: bool,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
    spinner: char,
) -> u16 {
    let (stripped, think_blocks) = extract_think_blocks(text);

    if stripped.trim().is_empty() && think_blocks.is_empty() {
        let content = if agent_running {
            format!("{} Thinking...", spinner)
        } else {
            "·".to_string()
        };
        let para = ratatui::widgets::Paragraph::new(ratatui::text::Line::raw(content).style(Style::default().fg(text_muted)))
            .style(Style::default().fg(text_muted));
        let para_area = Rect::new(margin_x, area.y + row, area.width - margin_x + area.x - 2, 1);
        para.render(para_area, buf);
        return 1;
    }

    let width = (area.width - margin_x + area.x - 2) as usize;
    let mut rendered = 0u16;

    if !think_blocks.is_empty() {
        rendered += 1;
    }

    for think in &think_blocks {
        if row + rendered >= max_rows {
            break;
        }
        let block_rows = render_think_block_box(think, area, row + rendered, margin_x, text_muted, wrap_cache, buf);
        rendered += block_rows;
    }

    if stripped.trim().is_empty() {
        return rendered;
    }

    if !think_blocks.is_empty() {
        rendered += 1;
    }

    let base_style = Style::default().fg(text_secondary);
    let stripped_trimmed = stripped.trim_start();
    let markdown_lines = render_text_content(stripped_trimmed, width, base_style);

    for (i, line) in markdown_lines.iter().enumerate() {
        let line_y = row + rendered + i as u16;
        if line_y >= max_rows {
            break;
        }
        buf.set_line(margin_x, area.y + line_y, line, area.width - margin_x + area.x - 2);
    }
    let text_rows = markdown_lines.len() as u16;
    rendered += text_rows;

    if cursor_visible && rendered > 0 {
        let cursor_y = area.y + row + rendered - 1;
        let last_line_text = markdown_lines.last().map(|l| l.to_string()).unwrap_or_default();
        let cursor_x = margin_x + (last_line_text.len() as u16).min(area.width - margin_x + area.x - 3);
        if cursor_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                cell.set_char('▊');
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
    rendered
}
