use ratatui::{buffer::Buffer, layout::Rect, style::{Style}, text::{Line, Span}, widgets::Widget};

use crate::components::message_list::WrapCache;
use crate::components::message_list::feed::ToolCall;
use crate::glyphs;
use crate::messages::MessageRegistry;
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
                // Unclosed think block - skip the opening <think> tag so it doesn't leak into main_text
                main_text.push_str(&text[last_end..start]);
                last_end = i + 7;
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
    while j + 8 <= bytes.len() {
        // Look for </think> (8 bytes) - with or without preceding newline
        if &bytes[j..j+8] == b"
</think>

" {
            // Return: (content_start, content_end, after_closing_tag)
            // content_start = start (after <think>), content_end = j (position of </think>)
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

/// Render thinking content with ┃ prefix (streaming case).
/// Returns the number of lines rendered.
fn render_thinking_stream(
    think_content: &str,
    area: Rect,
    row: u16,
    response_indent: u16,
    content_width: u16,
    text_muted: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
    buf: &mut Buffer,
) -> u16 {
    let mut rendered = 0u16;

    // Header: "┃  ◆ Thinking…"
    let header_text = format!("┃  {} Thinking…", glyphs::THOUGHT_MARKER);
    let header = Line::raw(header_text).style(Style::default().fg(text_muted));
    buf.set_line(response_indent, area.y + row, &header, content_width);
    rendered += 1;

    // Content lines with ┃  prefix
    let inner_width = (content_width - 4) as usize;
    let cleaned = think_content
        .replace("<think>", "")
        .replace("
</think>

", "")
        .trim()
        .to_string();
    let wrapped = wrap_cache.get_wrapped(&cleaned, inner_width);

    for line_text in wrapped {
        let line_y = row + rendered;
        if line_y >= area.height {
            break;
        }
        let content = format!("┃  {}", line_text);
        let line = Line::raw(content).style(Style::default().fg(text_muted));
        buf.set_line(response_indent, area.y + line_y, &line, content_width);
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
    _streaming_thinking_elapsed_ms: Option<u64>,
    _streaming_total_elapsed_ms: Option<u64>,
    _streaming_download_bytes: Option<u64>,
    streaming_think_content: Option<&str>,
) -> u16 {
    let (stripped, _think_blocks) = extract_think_blocks(text);

    // If we have streaming think content, we should show it (don't early return)
    let has_streaming_content = streaming_think_content.map_or(false, |s| !s.is_empty());

    if stripped.trim().is_empty() && !has_streaming_content {
        // Never show just a dot — always show meaningful status
        let content = if agent_running {
            format!("{} {}...", spinner, MessageRegistry::status_thinking())
        } else {
            format!("{} {}", glyphs::DOT, MessageRegistry::status_waiting())
        };
        let para = ratatui::widgets::Paragraph::new(Line::raw(content).style(Style::default().fg(text_muted)))
            .style(Style::default().fg(text_muted));
        let para_area = Rect::new(margin_x, area.y + row, area.width - margin_x + area.x - 2, 1);
        para.render(para_area, buf);
        return 1;
    }

    let content_width = area.width - margin_x + area.x - 2;
    // Response indent: 5 spaces from edge (margin_x + 3)
    let response_indent = margin_x + 3;
    let mut rendered = 0u16;

    // Always render thought indicator BEFORE answer (if thought_duration is provided)
    // This shows "◆ Thought for Xs" even when thoughts_collapsed is true
    let thought_rows = if let Some(duration) = thought_duration {
        let duration_text = MessageRegistry::thought_duration(duration);
        let line = Line::from(vec![
            Span::raw(format!("{} ", glyphs::THOUGHT_MARKER)).style(Style::default().fg(text_muted)),
            Span::raw(&duration_text).style(Style::default().fg(text_muted)),
        ]);
        buf.set_line(response_indent, area.y + row, &line, content_width);
        1
    } else {
        0
    };
    rendered += thought_rows;

    // Render tool calls inline (between thought indicator and markdown content)
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

    // STREAMING: Render thinking content directly with ┃ prefix
    if agent_running {
        if let Some(streaming_content) = streaming_think_content {
            if !streaming_content.is_empty() && row + rendered < max_rows {
                let block_rows = render_thinking_stream(
                    streaming_content,
                    area,
                    row + rendered,
                    response_indent,
                    content_width,
                    text_muted,
                    wrap_cache,
                    buf,
                );
                rendered += block_rows;
            }
        }
    } else {
        // NON-STREAMING: Render think blocks (if any) - compact form
        for think in &_think_blocks {
            if row + rendered >= max_rows {
                break;
            }
            let duration_text = extract_thought_duration(think);
            let header_text = format!("{} {}", glyphs::THOUGHT_MARKER, duration_text);
            let header = Line::raw(header_text).style(Style::default().fg(text_muted));
            buf.set_line(response_indent, area.y + row + rendered, &header, content_width);
            rendered += 1;
        }
    }

    // Don't render stripped thinking text when there is none
    // But still render "Turn completed" line if turn_complete is set
    if stripped.trim().is_empty() && turn_complete.is_none() && streaming_think_content.map_or(true, |s| s.is_empty()) {
        return rendered;
    }

    // Render full response text wrapped to content_width
    let stripped_trimmed = stripped.trim_start();
    let base_style = Style::default().fg(text_secondary);

    // Wrap full text using wrap_cache
    let wrapped = wrap_cache.get_wrapped(stripped_trimmed, content_width as usize);

    for (i, line_text) in wrapped.iter().enumerate() {
        if row + rendered >= max_rows {
            break;
        }

        // If last line and timestamp provided, render on SAME line (Grok style: text left, timestamp right)
        let is_last_line = i == wrapped.len() - 1;
        if is_last_line && timestamp.is_some() {
            let ts = timestamp.unwrap();
            let ts_len = ts.len() as u16;
            let line_len = line_text.len() as u16;

            // Pad line with spaces so timestamp appears right-aligned
            let padding = if line_len + ts_len + 1 < content_width {
                content_width - line_len - ts_len - 1
            } else {
                0
            };

            let line = Line::from(vec![
                Span::raw(line_text).style(base_style),
                Span::raw(" ".repeat(padding as usize)).style(base_style),
                Span::raw(ts).style(Style::default().fg(text_muted)),
            ]);
            buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
            rendered += 1;
        } else {
            let line = Line::raw(line_text).style(base_style);
            buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
            rendered += 1;
        }
    }

    // Render "Turn completed in Xs" on the NEXT line (only when turn is done, not streaming)
    if !agent_running {
        if let Some(elapsed) = turn_complete {
            if row + rendered < max_rows {
                let complete_text = MessageRegistry::turn_completed(elapsed as f32);
                let line = Line::raw(complete_text).style(Style::default().fg(text_muted));
                buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
                rendered += 1;
            }
        }
    }

    // Activity block: trailing `█` on the right edge while agent is still streaming.
    // Grok draws this on the last line of the in-progress assistant message.
    if agent_running && row + rendered < max_rows {
        let block_x = (area.x + area.width).saturating_sub(2);
        let block_y = area.y + row + rendered.saturating_sub(1);
        if let Some(cell) = buf.cell_mut((block_x, block_y)) {
            cell.set_char(glyphs::CURSOR_BLOCK);
            cell.set_style(Style::default().fg(_accent_bar_color));
        }
    }

    // Cursor placement at end of response line (5-space indent)
    // Calculate last line length from wrapped text
    if cursor_visible && !stripped_trimmed.is_empty() {
        let last_line_len = wrapped.last().map(|l| l.len()).unwrap_or(0);
        let cursor_y = area.y + row + rendered - 1;
        let cursor_x = response_indent + (last_line_len as u16).min(content_width);
        if cursor_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                cell.set_char(glyphs::CURSOR_BLOCK);
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
    rendered
}

/// Extract thought duration from think content (first line with "took" or similar)
fn extract_thought_duration(think_content: &str) -> String {
    // Look for patterns like "took X seconds", "X.Xs", etc.
    let lines: Vec<&str> = think_content.lines().collect();
    for line in &lines {
        // Try to find duration patterns
        if let Some(pos) = line.to_lowercase().find("took") {
            let rest = &line[pos + 4..];
            let trimmed = rest.trim();
            // Extract number and unit
            let chars = trimmed.chars().take(10).collect::<String>();
            if !chars.is_empty() {
                return format!("Thought for {}", chars.trim());
            }
        }
    }
    // Fallback: return first 30 chars of content
    let first_line = lines.first().unwrap_or(&"...");
    let preview = first_line.chars().take(30).collect::<String>();
    if preview.is_empty() {
        "...".to_string()
    } else {
        preview
    }
}
