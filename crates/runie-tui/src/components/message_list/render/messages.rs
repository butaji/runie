use std::fmt::Write;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Gauge, Paragraph, Widget},
};

use crate::components::message_list::PlanStatus;
use crate::glyphs;
use crate::messages::MessageRegistry;
use crate::tui::state::AnimationState;

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
    tool_calls: usize,
    tokens_used: Option<usize>,
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

    let mut tag = format!("[turn: {}", elapsed_str);
    if tool_calls > 0 {
        tag.push_str(&format!(", {}tc", tool_calls));
    }
    if let Some(tokens) = tokens_used {
        tag.push_str(&format!(", ⇣{}", format_token_count(tokens)));
    }
    tag.push(']');

    let tag_width = tag.len() as u16;
    let content_width = area.width - margin_x + area.x - 2;
    let x = if tag_width < content_width {
        margin_x + content_width - tag_width
    } else {
        margin_x
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
) -> u16 {
    let is_error = text.starts_with("Error:");
    let color = if is_error { error } else { text_muted };
    let prefix = if is_error { "! ".to_string() } else { format!("{} ", glyphs::DOT) };
    let line = Line::raw(format!("{}{}", prefix, text)).style(Style::default().fg(color));
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    1
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
) -> u16 {
    let line = Line::raw(format!("! {}", message)).style(Style::default().fg(error));
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    1
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

/// Render a tool running message
pub fn render_tool_running_msg(
    name: &str,
    args: &str,
    duration_ms: u64,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    buf: &mut Buffer,
    text_secondary: ratatui::style::Color,
    spinner: char,
    show_spinner: bool,
) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char(glyphs::TOOL_BULLET);
        cell.set_style(Style::default().fg(text_secondary));
    }
    let mut header = String::with_capacity(64);
    write!(header, "{} {}", name, args).ok();
    if show_spinner {
        write!(header, " {}", spinner).ok();
    }
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

/// Render a tool complete message
pub fn render_tool_complete_msg(
    name: &str,
    result: &str,
    lines: Option<&usize>,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    buf: &mut Buffer,
    success: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) -> u16 {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char(glyphs::CHECK_MARKER);
        cell.set_style(Style::default().fg(success));
    }
    let mut text = String::with_capacity(64);
    write!(text, "{} {}", name, result).ok();
    if let Some(l) = lines {
        write!(text, " ({} lines)", l).ok();
    }
    let line = Line::raw(text).style(Style::default().fg(text_muted));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
    1
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
    let available_width = area.width - text_x + area.x;

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
