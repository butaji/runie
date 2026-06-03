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
    while j < bytes.len() {
        // Look for newline followed by </think> (8 bytes)
        if j + 9 <= bytes.len() && bytes[j] == b'\n' && &bytes[j+1..j+9] == b"
</think>

" {
            return Some((start, j, j + 9));
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

/// Format download bytes for display (e.g., 1234 -> "1.2k", 1500000 -> "1.5M")
fn format_download_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}M", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}k", bytes as f64 / 1_000.0)
    } else {
        bytes.to_string()
    }
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
    spinner: char,
    streaming_thinking_elapsed_ms: Option<u64>,
    streaming_total_elapsed_ms: Option<u64>,
    streaming_download_bytes: Option<u64>,
) -> u16 {
    let mut rendered = 0u16;

    // Header: "┃  ◆ Thinking…"
    let header_text = "┃  ◆ Thinking…";
    let header = Line::raw(header_text).style(Style::default().fg(text_muted));
    buf.set_line(response_indent, area.y + row, &header, content_width);
    rendered += 1;

    // Content lines with ┃  prefix
    let inner_width = (content_width - 4) as usize;
    let cleaned = think_content
        .replace("<think>", "")
        .replace("</think>", "")
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

    // Bottom spinner line: "⠦ Thinking… X.Xs                      X.Xs ⇣XX.Xk [ ]"
    let bottom_y = row + rendered;
    if bottom_y < area.height {
        // Left: spinner + "Thinking…" + thinking elapsed
        let thinking_elapsed_secs = streaming_thinking_elapsed_ms
            .map(|ms| ms as f64 / 1000.0)
            .unwrap_or(0.0);
        let left_content = format!("{} Thinking… {:.1}s", spinner, thinking_elapsed_secs);
        let left_line = Line::raw(left_content).style(Style::default().fg(text_muted));
        buf.set_line(response_indent, area.y + bottom_y, &left_line, content_width);

        // Right: total elapsed + download bytes + status
        let total_elapsed_secs = streaming_total_elapsed_ms
            .map(|ms| ms as f64 / 1000.0)
            .unwrap_or(0.0);
        let download_str = streaming_download_bytes
            .map(format_download_bytes)
            .unwrap_or_else(|| "0".to_string());
        let right_text = format!(" {:.1}s ⇣{} [ ]", total_elapsed_secs, download_str);
        let right_len = right_text.len() as u16;
        let right_x = area.x + area.width - 1 - right_len;
        let right_line = Line::raw(right_text).style(Style::default().fg(text_muted));
        buf.set_line(right_x, area.y + bottom_y, &right_line, right_len);
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
    streaming_thinking_elapsed_ms: Option<u64>,
    streaming_total_elapsed_ms: Option<u64>,
    streaming_download_bytes: Option<u64>,
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

    // When thoughts_collapsed is true AND not streaming, render ONLY the compact duration line
    if thoughts_collapsed && !agent_running {
        let header_text = if let Some(duration) = thought_duration {
            format!("{} Thought for {:.1}s", glyphs::THOUGHT_MARKER, duration)
        } else {
            format!("{} Thinking...", glyphs::THOUGHT_MARKER)
        };
        let header = Line::raw(header_text).style(Style::default().fg(text_muted));
        buf.set_line(response_indent, area.y + row, &header, content_width);
        return 1;
    }

    // Render thought indicator BEFORE answer (if thought_duration is provided)
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
                    spinner,
                    streaming_thinking_elapsed_ms,
                    streaming_total_elapsed_ms,
                    streaming_download_bytes,
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

    // Grok-style: extract first sentence as plain text (no markdown)
    let stripped_trimmed = stripped.trim_start();
    let first_sentence = extract_first_sentence(stripped_trimmed);
    let base_style = Style::default().fg(text_secondary);
    let ts_len = timestamp.as_ref().map(|t| t.len() as u16).unwrap_or(0);

    // Render response text with timestamp on the SAME LINE (5-space indent)
    if let Some(ts) = timestamp {
        if ts_len > 0 {
            // Format: "response text                              timestamp"
            let response_len = first_sentence.len() as u16;
            let ts_x = response_indent + content_width.saturating_sub(ts_len);
            // Only render if we have room
            if response_len < ts_x.saturating_sub(response_indent) {
                let line = Line::raw(&first_sentence).style(base_style);
                buf.set_line(response_indent, area.y + row + rendered, &line, ts_x - response_indent);
                // Timestamp on same line, right-aligned
                let ts_line = Line::raw(ts).style(Style::default().fg(text_muted));
                buf.set_line(ts_x, area.y + row + rendered, &ts_line, ts_len);
            } else {
                // Not enough room, just show response
                let line = Line::raw(&first_sentence).style(base_style);
                buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
            }
        } else {
            let line = Line::raw(&first_sentence).style(base_style);
            buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
        }
    } else {
        let line = Line::raw(&first_sentence).style(base_style);
        buf.set_line(response_indent, area.y + row + rendered, &line, content_width);
    };
    rendered += 1;

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

    // Cursor placement at end of response line (5-space indent)
    if cursor_visible && !first_sentence.is_empty() {
        let cursor_y = area.y + row + rendered - 1;
        let cursor_x = response_indent + (first_sentence.len() as u16).min(area.width - response_indent + area.x - 3);
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

/// Extract first sentence from text for Grok-style single-line response
fn extract_first_sentence(text: &str) -> String {
    if let Some(end) = find_sentence_end(text) {
        return text[..=end].trim().to_string();
    }
    truncate_to_line(text)
}

fn find_sentence_end(text: &str) -> Option<usize> {
    for (i, c) in text.chars().enumerate() {
        if c == '.' || c == '!' || c == '?' {
            let next_i = i + 1;
            if next_i >= text.len() || text[next_i..].starts_with(' ') || text[next_i..].starts_with('\n') {
                return Some(i);
            }
        }
    }
    None
}

fn truncate_to_line(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or(text);
    let trimmed = first_line.trim();
    if trimmed.len() <= 80 {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(77).collect::<String>())
    }
}
