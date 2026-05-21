use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Modifier},
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct TopBar {
    pub repo_name: String,
    pub branch: String,
    pub path: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub agent_count: Option<usize>,
}

impl Default for TopBar {
    fn default() -> Self {
        Self {
            repo_name: String::new(),
            branch: String::new(),
            path: String::new(),
            checks_passed: None,
            checks_total: None,
            percentage: None,
            agent_count: None,
        }
    }
}

impl TopBar {
    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let x = area.x + 1;
        let text_secondary: ratatui::style::Color = theme.color("text.muted").into();
        let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
        let syntax_success: ratatui::style::Color = theme.color("success").into();

        let left_parts = self.build_left_parts(text_secondary, text_tertiary);
        if !left_parts.is_empty() {
            buf.set_line(x, area.y, &Line::from(left_parts), area.width - 2);
        }

        let right_parts = self.build_right_parts(syntax_success, text_secondary, text_tertiary);
        if !right_parts.is_empty() {
            render_right_info(buf, area, x, right_parts);
        }
    }

    fn build_left_parts(&self, text_secondary: ratatui::style::Color, text_tertiary: ratatui::style::Color) -> Vec<Span> {
        let mut parts = Vec::new();

        if !self.repo_name.is_empty() {
            parts.push(Span::styled(&self.repo_name, Style::default().fg(text_secondary)));
        }
        if !self.branch.is_empty() {
            if !parts.is_empty() {
                parts.push(Span::styled("/", Style::default().fg(text_secondary)));
            }
            parts.push(Span::styled(&self.branch, Style::default().fg(text_secondary)));
        }
        if !self.path.is_empty() {
            parts.push(Span::styled(format!(" {}", self.path), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)));
        }

        parts
    }

    fn build_right_parts(
        &self,
        syntax_success: ratatui::style::Color,
        text_secondary: ratatui::style::Color,
        text_tertiary: ratatui::style::Color,
    ) -> Vec<Span> {
        let mut parts = Vec::new();

        if let (Some(passed), Some(_total)) = (self.checks_passed, self.checks_total) {
            parts.push(Span::styled(format!("{} ", passed), Style::default().fg(syntax_success)));
            parts.push(Span::styled("✓ ", Style::default().fg(syntax_success)));
        }
        if let Some(pct) = self.percentage {
            parts.push(Span::styled(format!("{:.2}%", pct), Style::default().fg(text_secondary)));
            parts.push(Span::styled(format!(" {}", build_progress_bar(pct)), Style::default().fg(text_tertiary)));
        }

        parts
    }
}

fn build_progress_bar(pct: f32) -> String {
    let filled = (pct / 100.0 * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}

fn render_right_info(buf: &mut Buffer, area: Rect, left_x: u16, parts: Vec<Span>) {
    let right_line = Line::from(parts);
    let right_width: usize = right_line.spans.iter().map(|s| s.width()).sum();
    let right_x = area.x + area.width.saturating_sub(right_width as u16 + 1);
    if right_x > left_x {
        buf.set_line(right_x, area.y, &right_line, area.width);
    }
}
