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

        // Left side: repo_name/branch current_path
        if !self.repo_name.is_empty() || !self.branch.is_empty() {
            let mut left_parts: Vec<Span> = Vec::new();

            if !self.repo_name.is_empty() {
                left_parts.push(Span::styled(&self.repo_name, Style::default().fg(text_secondary)));
            }
            if !self.branch.is_empty() {
                left_parts.push(Span::styled("/", Style::default().fg(text_secondary)));
                left_parts.push(Span::styled(&self.branch, Style::default().fg(text_secondary)));
            }
            if !self.path.is_empty() {
                left_parts.push(Span::styled(format!(" {}", self.path), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)));
            }

            let line = Line::from(left_parts);
            buf.set_line(x, area.y, &line, area.width - 2);
        }

        // Right side: checks_passed ✓ percentage% with mini progress bar
        let mut right_parts: Vec<Span> = Vec::new();

        if let (Some(passed), Some(_total)) = (self.checks_passed, self.checks_total) {
            right_parts.push(Span::styled(format!("{} ", passed), Style::default().fg(syntax_success)));
            right_parts.push(Span::styled("✓ ", Style::default().fg(syntax_success)));
        }
        if let Some(pct) = self.percentage {
            right_parts.push(Span::styled(format!("{:.2}%", pct), Style::default().fg(text_secondary)));

            // Mini progress bar using unicode blocks
            let filled = (pct / 100.0 * 10.0).round() as usize;
            let empty = 10 - filled;
            let progress_bar = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));
            right_parts.push(Span::styled(format!(" {}", progress_bar), Style::default().fg(text_tertiary)));
        }

        if !right_parts.is_empty() {
            let right_line = Line::from(right_parts);
            let right_width: usize = right_line.spans.iter().map(|s| s.width()).sum();
            let right_x = area.x + area.width.saturating_sub(right_width as u16 + 1);
            if right_x > x {
                buf.set_line(right_x, area.y, &right_line, area.width);
            }
        }
    }
}
