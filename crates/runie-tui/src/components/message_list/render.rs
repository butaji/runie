use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Gauge, Widget},
};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use super::types::{MessageItem, PlanStatus};

/// Wrap text into lines respecting word boundaries
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

pub fn fill_background(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_base: ratatui::style::Color = theme.color("bg.base").into();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_base));
            }
        }
    }
}

pub fn should_show_cursor(
    animation: &AnimationState,
    agent_running: bool,
    absolute_idx: usize,
    total_messages: usize,
    msg: &MessageItem,
) -> bool {
    animation.streaming_cursor_visible
        && agent_running
        && absolute_idx == total_messages.saturating_sub(1)
        && matches!(msg, MessageItem::Assistant { .. })
}

/// Find the index of the most recent message that needs a spinner.
pub fn find_most_recent_spinner_index(messages: &[MessageItem]) -> Option<usize> {
    messages.iter().enumerate().rev().find(|(_, msg)| {
        matches!(msg,
            MessageItem::Thought { .. }
            | MessageItem::ToolRunning { .. }
            | MessageItem::PlanStep { status: PlanStatus::Active, .. }
            | MessageItem::Rewind { .. }
        )
    }).map(|(i, _)| i)
}

pub fn get_msg_type(msg: &MessageItem) -> &'static str {
    match msg {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Thought { .. } => "thought",
        MessageItem::ToolCall { .. } => "tool",
        MessageItem::Edit { .. } => "edit",
        MessageItem::System { .. } => "system",
        MessageItem::ToolRunning { .. } => "tool_running",
        MessageItem::ToolComplete { .. } => "tool_complete",
        MessageItem::PlanStep { .. } => "plan_step",
        MessageItem::Interrupt { .. } => "interrupt",
        MessageItem::Rewind { .. } => "rewind",
    }
}

pub fn render_single_msg(
    msg: &MessageItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
    spinner: char,
    cursor_visible: bool,
    show_spinner: bool,
    rewind_spinner: char,
    animation: &AnimationState,
) -> u16 {
    match msg {
        MessageItem::User { text, .. } => {
            render_user_msg(text, area, row, margin_x, text_x, max_rows, buf, theme, accent_primary)
        }
        MessageItem::Assistant { text, .. } => {
            render_assistant_msg(text, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, cursor_visible)
        }
        MessageItem::Thought { duration_secs } => {
            render_thought_msg(*duration_secs, area, row, margin_x, text_x, buf, text_muted, spinner, show_spinner)
        }
        MessageItem::ToolCall { name, args, result, is_error } => {
            render_tool_call_msg(name, args, result.as_deref(), *is_error, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, success, error)
        }
        MessageItem::Edit { filename, diff: _ } => {
            render_edit_msg(filename, area, row, margin_x, text_x, buf, text_secondary, code_path)
        }
        MessageItem::System { text } => {
            render_system_msg(text, area, row, margin_x, text_x, buf, text_muted)
        }
        MessageItem::ToolRunning { name, args, duration_ms } => {
            render_tool_running_msg(name, args, *duration_ms, area, row, margin_x, text_x, buf, text_secondary, spinner, show_spinner)
        }
        MessageItem::ToolComplete { name, result, lines } => {
            render_tool_complete_msg(name, result, lines.as_ref(), area, row, margin_x, text_x, buf, success, text_muted)
        }
        MessageItem::PlanStep { step, text, status } => {
            render_plan_step_msg(*step, text, status, area, row, margin_x, text_x, buf, text_dim, text_secondary, spinner, show_spinner)
        }
        MessageItem::Interrupt => {
            render_interrupt_msg(area, row, margin_x, text_x, buf, error, text_dim, animation)
        }
        MessageItem::Rewind { steps } => {
            render_rewind_msg(*steps, area, row, margin_x, text_x, buf, text_muted, rewind_spinner, show_spinner)
        }
    }
}

fn render_user_msg(
    text: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
) -> u16 {
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();

    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(8));
    let msg_height = wrapped.len() as u16;
    let total_height = msg_height + 2;

    draw_user_panel_bg(area, row, margin_x, total_height, buf, bg_panel);
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row + 1)) {
        cell.set_char('❯');
        cell.set_style(Style::default().fg(accent_primary).bg(bg_panel));
    }
    draw_user_text_lines(&wrapped, row, text_x, max_rows, area, buf, text_primary, bg_panel);

    total_height
}

fn draw_user_panel_bg(area: Rect, row: u16, margin_x: u16, total_height: u16, buf: &mut Buffer, bg_panel: ratatui::style::Color) {
    let panel_start_y = area.y + row;
    let panel_start_x = margin_x - 1;
    let panel_width = area.width - 2;
    for r in 0..total_height {
        if panel_start_y + r >= area.y + area.height { break; }
        for x in 0..panel_width {
            if panel_start_x + x < area.x + area.width {
                if let Some(cell) = buf.cell_mut((panel_start_x + x, panel_start_y + r)) {
                    cell.set_style(Style::default().bg(bg_panel));
                }
            }
        }
    }
}

fn draw_user_text_lines(wrapped: &[String], row: u16, text_x: u16, max_rows: u16, area: Rect, buf: &mut Buffer, text_primary: ratatui::style::Color, bg_panel: ratatui::style::Color) {
    for (i, line_text) in wrapped.iter().enumerate() {
        if row + 1 + i as u16 >= max_rows { break; }
        let line = Line::raw(line_text).style(Style::default().fg(text_primary).bg(bg_panel));
        buf.set_line(text_x, area.y + row + 1 + i as u16, &line, area.width - 6);
    }
}

fn render_assistant_msg(text: &str, area: Rect, row: u16, margin_x: u16, _text_x: u16, max_rows: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, text_muted: ratatui::style::Color, cursor_visible: bool) -> u16 {
    if text.is_empty() {
        let dot = Line::raw("·").style(Style::default().fg(text_muted));
        buf.set_line(margin_x, area.y + row, &dot, area.width - 2);
        return 1;
    }
    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(4));
    let msg_height = wrapped.len() as u16;
    for (i, line_text) in wrapped.iter().enumerate() {
        if row + i as u16 >= max_rows { break; }
        let line = Line::raw(line_text).style(Style::default().fg(text_secondary));
        buf.set_line(margin_x, area.y + row + i as u16, &line, area.width - 2);
    }
    if cursor_visible && !wrapped.is_empty() {
        let last_line_len = wrapped.last().map(|l| l.len()).unwrap_or(0) as u16;
        let cursor_x = margin_x + last_line_len;
        if cursor_x < area.x + area.width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, area.y + row + msg_height - 1)) {
                cell.set_char('▊');
                cell.set_style(Style::default().fg(text_secondary));
            }
        }
    }
    msg_height
}

fn render_thought_msg(duration_secs: f32, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_muted));
    }
    let spinner_str = if show_spinner { format!(" {}", spinner) } else { String::new() };
    let thought_text = format!("Thought for {:.1}s{}", duration_secs, spinner_str);
    let line = Line::raw(thought_text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

fn render_system_msg(text: &str, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_muted));
    }
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

fn render_tool_call_msg(
    name: &str,
    args: &str,
    result: Option<&str>,
    is_error: bool,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
) -> u16 {
    draw_tool_header(margin_x, text_x, area, row, buf, text_muted, text_secondary, name, args);
    let mut rendered = 1;
    if let Some(result_text) = result {
        rendered = draw_tool_result(result_text, is_error, area, row, text_x, max_rows, buf, text_muted, success, error);
    }
    rendered
}

fn draw_tool_header(margin_x: u16, text_x: u16, area: Rect, row: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, text_secondary: ratatui::style::Color, name: &str, args: &str) {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_muted));
    }
    let header = format!("{}({})", name, args);
    let line = Line::raw(header).style(Style::default().fg(text_secondary));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
}

fn draw_tool_result(result_text: &str, is_error: bool, area: Rect, row: u16, text_x: u16, max_rows: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, success: ratatui::style::Color, error: ratatui::style::Color) -> u16 {
    let result_y = row + 1;
    if result_y >= max_rows { return 1; }
    for (i, ch) in "  ".chars().enumerate() {
        if let Some(cell) = buf.cell_mut((text_x - 2 + i as u16, area.y + result_y)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(text_muted));
        }
    }
    if let Some(cell) = buf.cell_mut((text_x, area.y + result_y)) {
        cell.set_char('→');
        cell.set_style(Style::default().fg(text_muted));
    }
    if let Some(cell) = buf.cell_mut((text_x + 1, area.y + result_y)) {
        cell.set_char(if is_error { '×' } else { '✓' });
        cell.set_style(Style::default().fg(if is_error { error } else { success }));
    }
    let line = Line::raw(result_text).style(Style::default().fg(text_muted));
    buf.set_line(text_x + 3, area.y + result_y, &line, area.width - 7);
    2
}

fn render_edit_msg(filename: &str, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, code_path: ratatui::style::Color) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('◆');
        cell.set_style(Style::default().fg(text_secondary));
    }
    let edit_label = "Edit ";
    let filename_only = std::path::Path::new(filename).file_name().and_then(|n| n.to_str()).unwrap_or(filename);
    let edit_len = edit_label.len() as u16;
    for (i, ch) in edit_label.chars().enumerate() {
        if let Some(cell) = buf.cell_mut((text_x + i as u16, area.y + row)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(text_secondary));
        }
    }
    for (i, ch) in filename_only.chars().enumerate() {
        let x_pos = text_x + edit_len + i as u16;
        if x_pos < area.x + area.width {
            if let Some(cell) = buf.cell_mut((x_pos, area.y + row)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(code_path));
            }
        }
    }
    1
}

fn render_tool_running_msg(name: &str, args: &str, duration_ms: u64, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_secondary: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('●');
        cell.set_style(Style::default().fg(text_secondary));
    }
    let spinner_str = if show_spinner { format!(" {}", spinner) } else { String::new() };
    let header = format!("{} {}{}", name, args, spinner_str);
    let line = Line::raw(header).style(Style::default().fg(text_secondary));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    if duration_ms > 1000 {
        let bar_y = row + 1;
        let bar_x = text_x;
        let bar_width = 10u16;
        let ratio = duration_ms.min(10000) as f64 / 10000.0;
        let gauge_area = Rect::new(bar_x + 1, area.y + bar_y, bar_width, 1);
        Gauge::default()
            .ratio(ratio)
            .label("")
            .style(Style::default().fg(text_secondary))
            .render(gauge_area, buf);
        return 2;
    }
    1
}

fn render_tool_complete_msg(name: &str, result: &str, lines: Option<&usize>, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, success: ratatui::style::Color, text_muted: ratatui::style::Color) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('✓');
        cell.set_style(Style::default().fg(success));
    }
    let suffix = lines.map(|l| format!(" ({} lines)", l)).unwrap_or_default();
    let line = Line::raw(format!("{} {}{}", name, result, suffix)).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}

fn render_plan_step_msg(step: usize, text: &str, status: &PlanStatus, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_dim: ratatui::style::Color, text_secondary: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    match status {
        PlanStatus::Pending => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('▸');
                cell.set_style(Style::default().fg(text_dim));
            }
            let line = Line::raw(format!("{}. {} (pending)", step, text)).style(Style::default().fg(text_dim));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
        PlanStatus::Active => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(text_secondary));
            }
            if let Some(cell) = buf.cell_mut((margin_x + 1, area.y + row)) {
                cell.set_char('●');
                cell.set_style(Style::default().fg(text_secondary));
            }
            let pulse_char = if spinner == '⠋' || spinner == '⠹' || spinner == '⠴' || spinner == '⠧' || spinner == '⠏' { '▐' } else { ' ' };
            if pulse_char == '▐' {
                if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y + row)) {
                    cell.set_char('▐');
                    cell.set_style(Style::default().fg(text_secondary));
                }
            }
            let spinner_str = if show_spinner { format!(" {}", spinner) } else { String::new() };
            let line = Line::raw(format!("{}. {}{}", step, text, spinner_str)).style(Style::default().fg(text_secondary));
            buf.set_line(text_x + 1, area.y + row, &line, area.width - 5);
        }
        PlanStatus::Complete => {
            if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
                cell.set_char('✓');
                cell.set_style(Style::default().fg(text_secondary));
            }
            let line = Line::raw(format!("{}. {}", step, text)).style(Style::default().fg(text_secondary));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);
        }
    }
    1
}

fn render_interrupt_msg(area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, error: ratatui::style::Color, text_dim: ratatui::style::Color, animation: &AnimationState) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('✗');
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

fn render_rewind_msg(steps: usize, area: Rect, row: u16, margin_x: u16, text_x: u16, buf: &mut Buffer, text_muted: ratatui::style::Color, spinner: char, show_spinner: bool) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('↺');
        cell.set_style(Style::default().fg(text_muted));
    }
    let spinner_str = if show_spinner { format!(" {}", spinner) } else { String::new() };
    let line = Line::raw(format!("Rewinding...{} ({} steps)", spinner_str, steps)).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
}
