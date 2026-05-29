use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

pub mod builder;
pub use builder::*;

#[derive(Clone)]
pub struct AgentList {
    pub agents: Vec<AgentItem>,
    pub theme: ThemeWrapper,
}

#[derive(Debug, Clone)]
pub struct AgentItem {
    pub id: String,
    pub tag: String,
    pub tag_type: String,
    pub description: String,
    pub model: String,
    pub duration_secs: u64,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
    Waiting,
}

impl Default for AgentList {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            theme: ThemeWrapper::default(),
        }
    }
}

impl Widget for AgentList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = &self.theme;

        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let text_dim: ratatui::style::Color = theme.color("text.dim").into();
        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let success: ratatui::style::Color = theme.color("success").into();
        let error: ratatui::style::Color = theme.color("error").into();

        if area.width < 4 || area.height < 3 {
            return;
        }

        let inner_width = area.width.saturating_sub(2);

        render_border_top(area, buf, border_color, accent_primary, inner_width);
        fill_interior(area, buf, bg_panel, border_color);
        render_agents(area, buf, &self.agents, bg_panel, text_secondary, text_dim, accent_primary, success, error);
        render_border_bottom(area, buf, border_color);
    }
}

fn render_border_top(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color, accent_primary: ratatui::style::Color, inner_width: u16) {
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_char('╭');
        cell.set_style(Style::default().fg(border_color));
    }

    let header = " AGENTS ";
    let header_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    let header_len = header.len() as u16;
    let dashes = inner_width.saturating_sub(header_len);

    let mut x = area.x + 1;
    for ch in header.chars() {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char(ch);
            cell.set_style(header_style);
        }
        x += 1;
    }
    for _ in 0..dashes {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_color));
        }
        x += 1;
    }

    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y)) {
        cell.set_char('╮');
        cell.set_style(Style::default().fg(border_color));
    }
}

fn fill_interior(area: Rect, buf: &mut Buffer, bg_panel: ratatui::style::Color, border_color: ratatui::style::Color) {
    for y in (area.y + 1)..(area.y + area.height - 1) {
        for x in (area.x + 1)..(area.x + area.width - 1) {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg_panel));
            }
        }
    }

    for y in (area.y + 1)..(area.y + area.height - 1) {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_color));
        }
        if let Some(cell) = buf.cell_mut((area.x + area.width - 1, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_color));
        }
    }
}

fn render_agents(
    area: Rect,
    buf: &mut Buffer,
    agents: &[AgentItem],
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    accent_primary: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
) {
    let mut current_y = area.y + 1;
    let max_y = area.y + area.height - 1;

    for agent in agents {
        if current_y + 3 >= max_y {
            break;
        }

        let status_char = get_status_char(&agent.status, accent_primary, success, error, text_dim);
        let tag_color = get_tag_color(&agent.tag_type, accent_primary, text_dim);

        render_agent_status(area, buf, current_y, status_char, bg_panel);
        render_agent_tag(area, buf, current_y, &agent.tag, tag_color, bg_panel);
        render_agent_info(area, buf, current_y, &agent.description, &agent.model, agent.duration_secs, bg_panel, text_secondary, text_dim);

        current_y += 4;
        if current_y < max_y - 1 {
            render_separator(area, buf, current_y - 1, text_dim, bg_panel);
        }
    }
}

fn get_status_char(status: &AgentStatus, accent_primary: ratatui::style::Color, success: ratatui::style::Color, error: ratatui::style::Color, text_dim: ratatui::style::Color) -> (char, ratatui::style::Color) {
    match status {
        AgentStatus::Running => ('●', accent_primary),
        AgentStatus::Completed => ('✓', success),
        AgentStatus::Failed => ('✗', error),
        AgentStatus::Waiting => ('○', text_dim),
    }
}

fn get_tag_color(tag_type: &str, accent_primary: ratatui::style::Color, text_dim: ratatui::style::Color) -> ratatui::style::Color {
    match tag_type {
        "user" | "assistant" | "system" => accent_primary,
        _ => text_dim,
    }
}

fn render_agent_status(area: Rect, buf: &mut Buffer, y: u16, status: (char, ratatui::style::Color), bg_panel: ratatui::style::Color) {
    let (status_char, status_fg) = status;
    if let Some(cell) = buf.cell_mut((area.x + 2, y)) {
        cell.set_char(' ');
        cell.set_style(Style::default().bg(bg_panel));
    }
    if let Some(cell) = buf.cell_mut((area.x + 3, y)) {
        cell.set_char(status_char);
        cell.set_style(Style::default().fg(status_fg).bg(bg_panel));
    }
    if let Some(cell) = buf.cell_mut((area.x + 4, y)) {
        cell.set_char(' ');
        cell.set_style(Style::default().bg(bg_panel));
    }
}

fn render_agent_tag(area: Rect, buf: &mut Buffer, y: u16, tag: &str, tag_color: ratatui::style::Color, bg_panel: ratatui::style::Color) {
    let inner_width = area.width.saturating_sub(2);
    let tag_span = Span::styled(tag.to_string(), Style::default().fg(tag_color).add_modifier(Modifier::BOLD).bg(bg_panel));
    let tag_line = Line::from(vec![tag_span]);
    buf.set_line(area.x + 5, y, &tag_line, inner_width.saturating_sub(5));
}

fn render_agent_info(
    area: Rect,
    buf: &mut Buffer,
    y: u16,
    description: &str,
    model: &str,
    duration_secs: u64,
    bg_panel: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_dim: ratatui::style::Color,
) {
    let inner_width = area.width.saturating_sub(2);

    let desc_span = Span::styled(format!("  {}", description), Style::default().fg(text_secondary).bg(bg_panel));
    let desc_line = Line::from(vec![desc_span]);
    buf.set_line(area.x + 2, y + 1, &desc_line, inner_width.saturating_sub(2));

    let duration_str = if duration_secs >= 60 {
        format!("{}m", duration_secs / 60)
    } else {
        format!("{}s", duration_secs)
    };
    let meta_span = Span::styled(format!("  {} · {}", model, duration_str), Style::default().fg(text_dim).bg(bg_panel));
    let meta_line = Line::from(vec![meta_span]);
    buf.set_line(area.x + 2, y + 2, &meta_line, inner_width.saturating_sub(2));
}

fn render_separator(area: Rect, buf: &mut Buffer, y: u16, text_dim: ratatui::style::Color, bg_panel: ratatui::style::Color) {
    for sx in (area.x + 2)..(area.x + area.width - 2) {
        if let Some(cell) = buf.cell_mut((sx, y)) {
            cell.set_char('·');
            cell.set_style(Style::default().fg(text_dim).bg(bg_panel));
        }
    }
}

fn render_border_bottom(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color) {
    let bottom_y = area.y + area.height - 1;
    if let Some(cell) = buf.cell_mut((area.x, bottom_y)) {
        cell.set_char('╰');
        cell.set_style(Style::default().fg(border_color));
    }

    for x in (area.x + 1)..(area.x + area.width - 1) {
        if let Some(cell) = buf.cell_mut((x, bottom_y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_color));
        }
    }

    if let Some(cell) = buf.cell_mut((area.x + area.width - 1, bottom_y)) {
        cell.set_char('╯');
        cell.set_style(Style::default().fg(border_color));
    }
}
