//! Subagent Panel - parallel agent execution display

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Widget, Clear, Block, Borders},
};
use crate::style::box_chars::H;
use crate::style::selection::{STATUS_ACTIVE, STATUS_IDLE};
use crate::style::StyleSet;
use crate::theme::ThemeWrapper;

pub use crate::style::layout::SUBAGENT_PANEL_WIDTH;

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
    let theme = ThemeWrapper::default();
    let styles = StyleSet::from_theme(&theme);

    Clear.render(area, buf);
    Block::default().borders(Borders::ALL).border_style(Style::default().fg(styles.accent.fg.unwrap())).render(area, buf);

    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2));

    let y = render_subagent_header(panel, inner, buf, &styles);
    let y = render_subagent_grid(panel, inner, y, buf, &styles);
    let _y = render_subagent_list(panel, inner, y, buf, &styles);
    render_subagent_footer(inner, buf, &styles);
}

fn render_subagent_header(panel: &SubagentPanel, inner: Rect, buf: &mut Buffer, styles: &StyleSet) -> u16 {
    let progress_pct = format!("{:.2}%", panel.overall_progress * 100.0);
    let header = format!("{} {}", panel.context_name, progress_pct);
    let header_line = Line::styled(header, styles.text_primary.add_modifier(Modifier::BOLD));
    buf.set_line(inner.x, inner.y, &header_line, inner.width);
    let div: String = std::iter::repeat(H).take(inner.width as usize).collect();
    buf.set_line(inner.x, inner.y + 1, &Line::styled(div, styles.muted), inner.width);
    inner.y + 3
}

fn dot_for(status: SubagentStatus) -> char {
    match status {
        SubagentStatus::Active | SubagentStatus::Complete => STATUS_ACTIVE,
        SubagentStatus::Idle | SubagentStatus::Failed => STATUS_IDLE,
    }
}

fn render_subagent_grid(panel: &SubagentPanel, inner: Rect, mut y: u16, buf: &mut Buffer, styles: &StyleSet) -> u16 {
    let row1 = format!("{} {} {}",
        panel.subagents.get(0).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
        panel.subagents.get(1).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
        panel.subagents.get(2).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
    );
    buf.set_line(inner.x, y, &Line::styled(row1, styles.muted), inner.width);
    y += 1;

    let row2 = format!("{} {} {}",
        panel.subagents.get(3).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
        panel.subagents.get(4).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
        panel.subagents.get(5).map_or(STATUS_IDLE.to_string(), |s| dot_for(s.status).to_string()),
    );
    buf.set_line(inner.x, y, &Line::styled(row2, styles.muted), inner.width);
    y += 1;

    let legend = format!("{} = Active/working  {} = Idle/waiting", STATUS_ACTIVE, STATUS_IDLE);
    buf.set_line(inner.x, y, &Line::styled(legend, styles.muted), inner.width);
    y + 2
}

fn render_subagent_list(panel: &SubagentPanel, inner: Rect, mut y: u16, buf: &mut Buffer, styles: &StyleSet) -> u16 {
    for agent in &panel.subagents {
        if y >= inner.bottom().saturating_sub(2) {
            break;
        }
        let status_style = match agent.status {
            SubagentStatus::Active => styles.accent,
            SubagentStatus::Complete => styles.success,
            SubagentStatus::Failed => styles.error_msg,
            SubagentStatus::Idle => styles.muted,
        };
        let label = format!("[{}]", agent.label);
        let line = Line::from(vec![
            Span::styled(label, status_style),
            Span::styled(" ", Style::default()),
            Span::styled(&agent.description, styles.text_primary),
        ]);
        buf.set_line(inner.x, y, &line, inner.width);
        y += 1;
    }
    y
}

fn render_subagent_footer(inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    let footer_y = inner.bottom().saturating_sub(1);
    let footer = "Ctrl+Shift+A: toggle  │  Esc: close";
    buf.set_line(inner.x, footer_y, &Line::styled(footer, styles.muted), inner.width);
}