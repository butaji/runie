//! Subagent Panel - parallel agent execution display

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Widget, Clear, Block, Borders},
};

pub const SUBAGENT_PANEL_WIDTH: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubagentStatus {
    Idle,
    Active,
    Complete,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Subagent {
    pub label: String,
    pub description: String,
    pub status: SubagentStatus,
    pub progress: f64,
}

#[derive(Debug, Clone)]
pub struct SubagentPanel {
    pub visible: bool,
    pub subagents: Vec<Subagent>,
    pub context_name: String,
    pub overall_progress: f64,
}

impl SubagentPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            subagents: vec![],
            context_name: String::new(),
            overall_progress: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn calculate_overall_progress(&mut self) {
        if self.subagents.is_empty() {
            self.overall_progress = 0.0;
            return;
        }
        let total: f64 = self.subagents.iter().map(|s| s.progress).sum();
        self.overall_progress = total / self.subagents.len() as f64;
    }
}

impl Default for SubagentPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &SubagentPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        render_subagent_panel(self, area, buf);
    }
}

fn render_subagent_panel(panel: &SubagentPanel, area: Rect, buf: &mut Buffer) {
    let muted = Color::Rgb(140, 140, 160);
    let accent = Color::Rgb(41, 198, 190);

    Clear.render(area, buf);
    Block::default().borders(Borders::ALL).border_style(Style::default().fg(accent)).render(area, buf);

    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2));

    let y = render_subagent_header(panel, inner, buf, muted);
    let y = render_subagent_grid(panel, inner, y, buf, muted);
    let _y = render_subagent_list(panel, inner, y, buf);
    render_subagent_footer(inner, buf, muted);
}

fn render_subagent_header(panel: &SubagentPanel, inner: Rect, buf: &mut Buffer, muted: Color) -> u16 {
    let fg = Color::Rgb(225, 225, 225);
    let progress_pct = format!("{:.2}%", panel.overall_progress * 100.0);
    let header = format!("{} {}", panel.context_name, progress_pct);
    let header_line = Line::styled(header, Style::default().fg(fg).add_modifier(Modifier::BOLD));
    buf.set_line(inner.x, inner.y, &header_line, inner.width);
    let div = "─".repeat(inner.width as usize);
    buf.set_line(inner.x, inner.y + 1, &Line::styled(div, Style::default().fg(muted)), inner.width);
    inner.y + 3
}

fn dot_for(status: SubagentStatus) -> &'static str {
    match status {
        SubagentStatus::Active | SubagentStatus::Complete => "●",
        SubagentStatus::Idle | SubagentStatus::Failed => "○",
    }
}

fn render_subagent_grid(panel: &SubagentPanel, inner: Rect, mut y: u16, buf: &mut Buffer, muted: Color) -> u16 {
    let row1 = format!("{} {} {}",
        panel.subagents.get(0).map_or("○", |s| dot_for(s.status)),
        panel.subagents.get(1).map_or("○", |s| dot_for(s.status)),
        panel.subagents.get(2).map_or("○", |s| dot_for(s.status)),
    );
    buf.set_line(inner.x, y, &Line::styled(row1, Style::default().fg(muted)), inner.width);
    y += 1;

    let row2 = format!("{} {} {}",
        panel.subagents.get(3).map_or("○", |s| dot_for(s.status)),
        panel.subagents.get(4).map_or("○", |s| dot_for(s.status)),
        panel.subagents.get(5).map_or("○", |s| dot_for(s.status)),
    );
    buf.set_line(inner.x, y, &Line::styled(row2, Style::default().fg(muted)), inner.width);
    y += 1;

    let legend = "● = Active/working  ○ = Idle/waiting";
    buf.set_line(inner.x, y, &Line::styled(legend, Style::default().fg(muted)), inner.width);
    y + 2
}

fn render_subagent_list(panel: &SubagentPanel, inner: Rect, mut y: u16, buf: &mut Buffer) -> u16 {
    let fg = Color::Rgb(225, 225, 225);
    let muted = Color::Rgb(140, 140, 160);
    let accent = Color::Rgb(41, 198, 190);
    let success = Color::Rgb(52, 211, 153);
    let error = Color::Rgb(248, 113, 113);

    for agent in &panel.subagents {
        if y >= inner.bottom().saturating_sub(2) {
            break;
        }
        let status_color = match agent.status {
            SubagentStatus::Active => accent,
            SubagentStatus::Complete => success,
            SubagentStatus::Failed => error,
            SubagentStatus::Idle => muted,
        };
        let label = format!("[{}]", agent.label);
        let line = Line::from(vec![
            Span::styled(label, Style::default().fg(status_color)),
            Span::styled(" ", Style::default()),
            Span::styled(&agent.description, Style::default().fg(fg)),
        ]);
        buf.set_line(inner.x, y, &line, inner.width);
        y += 1;
    }
    y
}

fn render_subagent_footer(inner: Rect, buf: &mut Buffer, muted: Color) {
    let footer_y = inner.bottom().saturating_sub(1);
    let footer = "Ctrl+Shift+A: toggle  │  Esc: close";
    buf.set_line(inner.x, footer_y, &Line::styled(footer, Style::default().fg(muted)), inner.width);
}