use std::fmt::Write;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::components::message_list::PlanStatus;
use crate::components::message_list::WrapCache;
use crate::glyphs;
use crate::messages::MessageRegistry;
use crate::tui::state::AnimationState;

use crate::components::message_list::feed::FeedItem;

/// Render empty lines between feed items based on context.
/// - SystemNotice → UserMessage: 2 blank lines
/// - AssistantMessage → Separator: 1 blank line (grouped together)
/// - All other transitions: 2 blank lines
pub fn render_item_separator(
    _area: Rect,
    _row: u16,
    _buf: &mut Buffer,
    _color: Color,
    current_item: &FeedItem,
    next_item: &FeedItem,
) -> u16 {
    match (current_item, next_item) {
        (FeedItem::AssistantMessage { .. }, FeedItem::Separator { .. }) => 1,
        (FeedItem::SystemNotice { .. }, _) => 2,
        _ => 2,
    }
}

// ============================================================================
// Message Renderers
// ============================================================================

/// Render a thought duration message
pub fn render_thought_msg(
    duration_secs: f32,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    _spinner: char,
    _show_spinner: bool,
) -> u16 {
    let text = format!("{} {}", glyphs::THOUGHT_MARKER, MessageRegistry::thought_duration(duration_secs));
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    1
}

/// Render a separator with timing info
pub fn render_separator(
    elapsed_secs: u64,
    _tool_calls: usize,
    tokens_used: Option<usize>,
    success: bool,
    area: Rect,
    row: u16,
    margin_x: u16,
    buf: &mut Buffer,
    text_dim: ratatui::style::Color,
) -> u16 {
    let elapsed_str = if elapsed_secs < 60 {
        format!("{}s", elapsed_secs)
    } else if elapsed_secs < 3600 {
        format!("{}m {:02}s", elapsed_secs / 60, elapsed_secs % 60)
    } else {
        format!("{}h {:02}m", elapsed_secs / 3600, (elapsed_secs % 3600) / 60)
    };

    let mut parts = vec![elapsed_str.clone()];
    if let Some(tokens) = tokens_used {
        parts.push(format!("⇣{}", format_token_count(tokens)));
    }
    parts.push(if success { "[✓]".to_string() } else { "[✗]".to_string() });
    let tag = parts.join(" ");

    let tag_width = tag.len() as u16;
    let content_width = area.width - margin_x + area.x - 2;
    // margin_x + 3 = area.x + 5 = 5 spaces from screen edge (Grok-style)
    let indent = margin_x + 3;
    let x = if tag_width < content_width.saturating_sub(5) {
        indent + content_width.saturating_sub(5) - tag_width
    } else {
        indent
    };
    let line = Line::raw(tag).style(Style::default().fg(text_dim));
    buf.set_line(x, area.y + row, &line, tag_width);
    1
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Render a system message
pub fn render_system_msg(
    text: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    error: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let is_error = text.starts_with("Error:");
    let color = if is_error { error } else { text_muted };
    // 3 leading spaces + prefix = 5 chars before text (margin_x + 2 + 3 = area.x + 5 = 5 spaces from edge)
    let prefix = if is_error { "   ! " } else { "   ◆ " };
    let prefix_len = prefix.len();
    let content_width = (area.width - margin_x + area.x - 2) as usize;
    let text_width = content_width.saturating_sub(prefix_len);
    let wrapped = wrap_cache.get_wrapped(text, text_width);
    for (i, line_text) in wrapped.iter().enumerate() {
        let line_y = area.y + row + i as u16;
        if line_y >= area.bottom() {
            break;
        }
        let full_line = format!("{}{}", prefix, line_text);
        let line = Line::raw(full_line).style(Style::default().fg(color));
        buf.set_line(margin_x, line_y, &line, content_width as u16);
    }
    wrapped.len() as u16
}

/// Render an error message
pub fn render_error_msg(
    message: &str,
    _recoverable: bool,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    error: ratatui::style::Color,
    _text_muted: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let prefix = "! ";
    let prefix_len = prefix.len();
    let content_width = (area.width - margin_x + area.x - 2) as usize;
    let text_width = content_width.saturating_sub(prefix_len);
    let wrapped = wrap_cache.get_wrapped(message, text_width);
    for (i, line_text) in wrapped.iter().enumerate() {
        let line_y = area.y + row + i as u16;
        if line_y >= area.bottom() {
            break;
        }
        let full_line = format!("{}{}", prefix, line_text);
        let line = Line::raw(full_line).style(Style::default().fg(error));
        buf.set_line(margin_x, line_y, &line, content_width as u16);
    }
    wrapped.len() as u16
}

/// Render an edit message
pub fn render_edit_msg(
    filename: &str,
    _diff: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    _text_secondary: ratatui::style::Color,
    code_path: ratatui::style::Color,
) -> u16 {
    let text = format!("{} Edit {}", glyphs::THOUGHT_MARKER, filename);
    let line = Line::raw(text).style(Style::default().fg(code_path));
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    1
}

/// Render a tool running message (Grok-style block)
pub fn render_tool_running_msg(
    name: &str,
    _args: &str,
    duration_ms: u64,
    total_elapsed_ms: u64,
    download_bytes: u64,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    spinner: char,
    _show_spinner: bool,
) -> u16 {
    // Grok-style: "     ⠧ Thinking… 1.5s                                           8.0s ⇣23.2k [✗]"
    // Left side: spinner + "Thinking…" + duration
    // Right side: total_elapsed + transfer + status (empty brackets while running)

    let tool_bar_color = ratatui::style::Color::Rgb(0x6B, 0x50, 0xFF); // Purple accent
    let indent = 3; // margin_x + 3 = 5 spaces from screen edge (accounting for 2-space padding in margin_x)

    let content_x = margin_x + indent;
    let elapsed_secs = duration_ms as f64 / 1000.0;

    // Build left content: spinner + name + "…" + tool_duration
    let mut left_content = String::with_capacity(64);
    write!(left_content, "{} {}… {:.1}s", spinner, name, elapsed_secs).ok();

    let left_line = Line::raw(left_content).style(Style::default().fg(tool_bar_color));
    buf.set_line(content_x, area.y + row, &left_line, area.width - 4);

    // Right side: total elapsed + transfer bytes + status (empty brackets while running)
    let total_elapsed_secs = total_elapsed_ms as f64 / 1000.0;
    let transfer_str = format_transfer_bytes(download_bytes as usize);
    let right_text = format!(" {:.1}s ⇣{} [ ]", total_elapsed_secs, transfer_str);
    let right_len = right_text.len() as u16;
    let right_x = area.x + area.width - 1 - right_len;
    let right_line = Line::raw(right_text).style(Style::default().fg(text_secondary));
    buf.set_line(right_x, area.y + row, &right_line, right_len);

    1
}

/// Render a tool complete message (Grok-style block)
pub fn render_tool_complete_msg(
    name: &str,
    result: &str,
    lines: Option<&usize>,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    buf: &mut Buffer,
    success: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) -> u16 {
    // Grok-style: "⠴ Run List `.` 1.8s                         5.7s ⇣21.2k [✗]"
    // Left: checkmark + name + args + result preview
    // Right: elapsed + transfer bytes + status

    let error_color = ratatui::style::Color::Rgb(0xEB, 0x42, 0x68);

    // Determine if result looks like an error
    let is_error = result.starts_with("Error:") || result.starts_with("error:") || result.starts_with("❌");
    let status_color = if is_error { error_color } else { success };
    let status_icon = if is_error { "[✗]" } else { "[✓]" };

    // 5-space indent (Grok-style)
    let indent = 3; // margin_x + 3 = 5 spaces from screen edge (accounting for 2-space padding in margin_x)
    let content_x = margin_x + indent;

    // Build left content: checkmark + name + args + result preview
    let compact_args = super::tool::format_tool_args_compact(result);
    let content = if compact_args.is_empty() {
        format!("{} {}", glyphs::CHECK_MARKER, name)
    } else {
        format!("{} {} → {}", glyphs::CHECK_MARKER, name, compact_args)
    };

    let left_line = Line::raw(content).style(Style::default().fg(status_color));
    buf.set_line(content_x, area.y + row, &left_line, area.width - 4);

    // Right side: transfer bytes + status
    let result_bytes = result.len();
    let transfer_str = format_transfer_bytes(result_bytes);
    let right_text = format!(" ⇣{} {}", transfer_str, status_icon);
    let right_len = right_text.len() as u16;
    let right_x = area.x + area.width - 1 - right_len;
    let right_line = Line::raw(right_text).style(Style::default().fg(text_muted));
    buf.set_line(right_x, area.y + row, &right_line, right_len);

    // If there are lines, show on second line
    if let Some(l) = lines {
        if *l > 1 {
            let line2_text = format!("  ({} lines)", l);
            let line2 = Line::raw(line2_text).style(Style::default().fg(text_muted));
            buf.set_line(content_x, area.y + row + 1, &line2, area.width - 4);
            return 2;
        }
    }
    1
}

fn format_transfer_bytes(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}M", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}k", bytes as f64 / 1_000.0)
    } else {
        bytes.to_string()
    }
}

/// Render a plan step message
pub fn render_plan_step_msg(
    step: usize,
    text: &str,
    status: &PlanStatus,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    buf: &mut Buffer,
    text_dim: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    spinner: char,
    show_spinner: bool,
) -> u16 {
    match status {
        PlanStatus::Pending => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char(glyphs::PLAN_PENDING);
                cell.set_style(Style::default().fg(text_dim));
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {} (pending)", step, text).ok();
            let line = Line::raw(line_text).style(Style::default().fg(text_dim));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
        PlanStatus::Active => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char(glyphs::PLAN_ACTIVE);
                cell.set_style(Style::default().fg(text_secondary));
            }
            if let Some(cell) = buf.cell_mut((margin_x + 1, area.y + row)) {
                cell.set_char(glyphs::TOOL_BULLET);
                cell.set_style(Style::default().fg(text_secondary));
            }
            let pulse_char = if spinner == '⠋' || spinner == '⠹' || spinner == '⠴' || spinner == '⠧' || spinner == '⠏' {
                glyphs::PULSE_FILL
            } else {
                ' '
            };
            if pulse_char == glyphs::PULSE_FILL {
                if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y + row)) {
                    cell.set_char(glyphs::PULSE_FILL);
                    cell.set_style(Style::default().fg(text_secondary));
                }
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {}", step, text).ok();
            if show_spinner {
                write!(line_text, " {}", spinner).ok();
            }
            let line = Line::raw(line_text).style(Style::default().fg(text_secondary));
            buf.set_line(text_x + 1, area.y + row, &line, area.width - 5);
        }
        PlanStatus::Complete => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char(glyphs::CHECK_MARKER);
                cell.set_style(Style::default().fg(text_secondary));
            }
            let mut line_text = String::with_capacity(32);
            write!(line_text, "{}. {}", step, text).ok();
            let line = Line::raw(line_text).style(Style::default().fg(text_secondary));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
    }
    1
}

/// Render an interrupt message
pub fn render_interrupt_msg(
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    buf: &mut Buffer,
    error: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    animation: &AnimationState,
) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char(glyphs::INTERRUPT);
        cell.set_style(Style::default().fg(error));
    }
    let style = if let Some(start) = animation.interrupt_fade_start {
        let elapsed = start.elapsed().as_millis() as f32;
        let fade_ms = 500.0;
        if elapsed >= fade_ms {
            Style::default().fg(text_dim)
        } else {
            Style::default().fg(error)
        }
    } else {
        Style::default().fg(error)
    };
    let line = Line::raw("Interrupted").style(style);
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

/// Render a rewind message
pub fn render_rewind_msg(
    steps: usize,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    spinner: char,
    show_spinner: bool,
) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char(glyphs::REWIND);
        cell.set_style(Style::default().fg(text_muted));
    }
    let mut text = String::with_capacity(32);
    write!(text, "Rewinding...").ok();
    if show_spinner {
        write!(text, " {}", spinner).ok();
    }
    write!(text, " ({} steps)", steps).ok();
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

/// Render the empty-state welcome message
pub fn render_empty_state(
    area: Rect,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    text_x: u16,
) {
    let center_y = area.height / 2;
    let available_width = area.width.saturating_sub(text_x).saturating_add(area.x);

    let title = Paragraph::new(Line::raw("runie").style(Style::default().fg(text_dim).add_modifier(Modifier::BOLD)))
        .style(Style::default().fg(text_dim));
    title.render(Rect::new(text_x, center_y.saturating_sub(3), available_width, 1), buf);

    let tagline = Paragraph::new(Line::raw("Your coding companion").style(Style::default().fg(text_muted)))
        .style(Style::default().fg(text_muted));
    tagline.render(Rect::new(text_x, center_y.saturating_sub(2), available_width, 1), buf);

    let cta = Paragraph::new(Line::raw("Type a message and press Enter to start").style(Style::default().fg(text_muted)))
        .style(Style::default().fg(text_muted));
    cta.render(Rect::new(text_x, center_y, available_width, 1), buf);

    let hint1 = Paragraph::new(Line::raw("Press ^k for commands · ^b for sidebar · ^q to quit").style(Style::default().fg(text_dim)))
        .style(Style::default().fg(text_dim));
    hint1.render(Rect::new(text_x, center_y.saturating_add(1), available_width, 1), buf);
}
