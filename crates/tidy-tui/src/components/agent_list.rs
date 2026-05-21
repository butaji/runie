use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct AgentList {
    pub agents: Vec<AgentItem>,
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
        Self { agents: Vec::new() }
    }
}

impl Widget for AgentList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();

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

        // ──── Top border ────
        buf.get_mut(area.x, area.y).set_char('╭');
        buf.get_mut(area.x, area.y).set_style(Style::default().fg(border_color));

        let header = " AGENTS ";
        let header_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
        let header_len = header.len() as u16;
        let dashes = inner_width.saturating_sub(header_len);

        // Header text
        let mut x = area.x + 1;
        for ch in header.chars() {
            buf.get_mut(x, area.y).set_char(ch);
            buf.get_mut(x, area.y).set_style(header_style);
            x += 1;
        }
        // Remaining top border dashes
        for _ in 0..dashes {
            buf.get_mut(x, area.y).set_char('─');
            buf.get_mut(x, area.y).set_style(Style::default().fg(border_color));
            x += 1;
        }

        buf.get_mut(area.x + area.width - 1, area.y).set_char('╮');
        buf.get_mut(area.x + area.width - 1, area.y).set_style(Style::default().fg(border_color));

        // ──── Interior ────
        // Fill with bg.panel
        for y in (area.y + 1)..(area.y + area.height - 1) {
            for x in (area.x + 1)..(area.x + area.width - 1) {
                buf.get_mut(x, y).set_style(Style::default().bg(bg_panel));
            }
        }

        // Left and right borders
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(area.x, y).set_char('│');
            buf.get_mut(area.x, y).set_style(Style::default().fg(border_color));
            buf.get_mut(area.x + area.width - 1, y).set_char('│');
            buf.get_mut(area.x + area.width - 1, y).set_style(Style::default().fg(border_color));
        }

        // ──── Agent items ────
        let mut current_y = area.y + 1;
        let max_y = area.y + area.height - 1;

        for agent in &self.agents {
            if current_y + 3 >= max_y {
                break;
            }

            // Status icon + tag on one line
            let (status_char, status_fg) = match agent.status {
                AgentStatus::Running => ('●', accent_primary),
                AgentStatus::Completed => ('✓', success),
                AgentStatus::Failed => ('✗', error),
                AgentStatus::Waiting => ('○', text_dim),
            };

            let tag_color = match agent.tag_type.as_str() {
                "user" | "assistant" => accent_primary,
                "system" => accent_primary,
                _ => text_dim,
            };

            // Line 1: icon + tag
            let y1 = current_y;
            buf.get_mut(area.x + 2, y1).set_char(' ');
            buf.get_mut(area.x + 2, y1).set_style(Style::default().bg(bg_panel));
            buf.get_mut(area.x + 3, y1).set_char(status_char);
            buf.get_mut(area.x + 3, y1).set_style(Style::default().fg(status_fg).bg(bg_panel));
            buf.get_mut(area.x + 4, y1).set_char(' ');
            buf.get_mut(area.x + 4, y1).set_style(Style::default().bg(bg_panel));

            let tag_span = Span::styled(
                agent.tag.clone(),
                Style::default().fg(tag_color).add_modifier(Modifier::BOLD).bg(bg_panel),
            );
            let tag_line = Line::from(vec![tag_span]);
            buf.set_line(area.x + 5, y1, &tag_line, inner_width.saturating_sub(5));

            // Line 2: description
            let y2 = current_y + 1;
            let desc_span = Span::styled(
                format!("  {}", agent.description),
                Style::default().fg(text_secondary).bg(bg_panel),
            );
            let desc_line = Line::from(vec![desc_span]);
            buf.set_line(area.x + 2, y2, &desc_line, inner_width.saturating_sub(2));

            // Line 3: model + duration
            let y3 = current_y + 2;
            let duration_str = if agent.duration_secs >= 60 {
                format!("{}m", agent.duration_secs / 60)
            } else {
                format!("{}s", agent.duration_secs)
            };
            let meta_span = Span::styled(
                format!("  {} · {}", agent.model, duration_str),
                Style::default().fg(text_dim).bg(bg_panel),
            );
            let meta_line = Line::from(vec![meta_span]);
            buf.set_line(area.x + 2, y3, &meta_line, inner_width.saturating_sub(2));

            // Separator line after agent (if not last and room exists)
            current_y += 4;
            if current_y < max_y - 1 {
                let sep_y = current_y - 1;
                for sx in (area.x + 2)..(area.x + area.width - 2) {
                    buf.get_mut(sx, sep_y).set_char('·');
                    buf.get_mut(sx, sep_y).set_style(Style::default().fg(text_dim).bg(bg_panel));
                }
            }
        }

        // ──── Bottom border ────
        let bottom_y = area.y + area.height - 1;
        buf.get_mut(area.x, bottom_y).set_char('╰');
        buf.get_mut(area.x, bottom_y).set_style(Style::default().fg(border_color));

        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, bottom_y).set_char('─');
            buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
        }

        buf.get_mut(area.x + area.width - 1, bottom_y).set_char('╯');
        buf.get_mut(area.x + area.width - 1, bottom_y).set_style(Style::default().fg(border_color));
    }
}
