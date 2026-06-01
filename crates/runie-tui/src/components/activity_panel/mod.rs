//! Activity Panel - shows running tools/agents in a right-side panel.

use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line, widgets::Widget};
use crate::theme::ThemeColors;
use crate::components::status_bar::BackgroundJob;

/// Activity panel width in characters
pub const ACTIVITY_PANEL_WIDTH: u16 = 30;

/// Activity panel state
#[derive(Debug, Clone, Default)]
pub struct ActivityPanel {
    pub running_jobs: Vec<BackgroundJob>,
}

impl ActivityPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_jobs(jobs: Vec<BackgroundJob>) -> Self {
        Self { running_jobs: jobs }
    }
}

/// Check if activity panel should be visible based on screen width
pub fn should_show_activity_panel(screen_width: u16) -> bool {
    screen_width >= 100
}

fn render_background(buf: &mut Buffer, area: Rect, bg: ratatui::style::Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            }
        }
    }
}

fn render_border(buf: &mut Buffer, area: Rect, border_color: ratatui::style::Color) {
    for x in area.x..area.right() {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_color));
        }
        if let Some(cell) = buf.cell_mut((x, area.bottom().saturating_sub(1))) {
            cell.set_char('─');
            cell.set_style(Style::default().fg(border_color));
        }
    }
    for y in area.y..area.bottom() {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_color));
        }
        if let Some(cell) = buf.cell_mut((area.right().saturating_sub(1), y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(border_color));
        }
    }
}

fn render_jobs(panel: &ActivityPanel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let mut y = area.y + 2;
    let max_y = area.bottom().saturating_sub(2);

    if panel.running_jobs.is_empty() {
        let empty_style = Style::default().fg(colors.text_dim);
        let empty_line = Line::styled("No active tasks", empty_style);
        buf.set_line(area.x + 1, y, &empty_line, area.width.saturating_sub(2));
        return;
    }

    for job in &panel.running_jobs {
        if y > max_y {
            break;
        }
        let (indicator, indicator_color) = match job.status {
            crate::components::status_bar::JobStatus::Running => ('●', colors.accent_primary),
            crate::components::status_bar::JobStatus::Complete => ('✓', colors.success),
            crate::components::status_bar::JobStatus::Failed => ('✗', colors.error),
        };
        let indicator_style = Style::default().fg(indicator_color);
        let job_style = Style::default().fg(colors.text_secondary);
        use ratatui::text::Span;
        let line = Line::from(vec![
            Span::styled(format!("{} ", indicator), indicator_style),
            Span::styled(&job.name, job_style),
        ]);
        buf.set_line(area.x + 1, y, &line, area.width.saturating_sub(2));
        y += 1;
    }
}

/// Render the activity panel
pub fn render_activity_panel(panel: &ActivityPanel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    if area.width < 4 || area.height < 3 {
        return;
    }
    render_background(buf, area, colors.bg_panel);
    // Left border uses accent_primary, other borders use border_unfocused
    render_border(buf, area, colors.border_unfocused);
    // Draw accent left border
    for y in area.y..area.bottom() {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(colors.accent_primary));
        }
    }
    let title_style = Style::default().fg(colors.accent_primary).add_modifier(ratatui::style::Modifier::BOLD);
    let title_line = Line::styled("Activity", title_style);
    buf.set_line(area.x + 1, area.y, &title_line, 8);
    render_jobs(panel, area, buf, colors);
}

impl Widget for &ActivityPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let colors = ThemeColors::from(&crate::theme::ThemeWrapper::default());
        render_activity_panel(&self, area, buf, &colors);
    }
}
