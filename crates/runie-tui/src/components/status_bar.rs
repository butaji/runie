use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::{ThemeColors, ThemeWrapper};
use crate::tui::state::TuiMode;
use crate::tui::view_models::StatusBarViewModel;

pub mod builder;
pub use builder::*;

#[derive(Clone)]
pub struct StatusBar {
    pub items: Vec<StatusItem>,
    pub theme: ThemeWrapper,
    pub background_jobs: Vec<BackgroundJob>,
}

#[derive(Debug, Clone)]
pub struct StatusItem {
    pub key: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct BackgroundJob {
    pub name: String,
    pub status: JobStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running,
    Complete,
    Failed,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            items: vec![
                StatusItem { key: "Enter".to_string(), description: "send".to_string() },
                StatusItem { key: "^b".to_string(), description: "sidebar".to_string() },
                StatusItem { key: "^k".to_string(), description: "cmd".to_string() },
                StatusItem { key: "^q".to_string(), description: "quit".to_string() },
            ],
            theme: ThemeWrapper::default(),
            background_jobs: Vec::new(),
        }
    }
}

impl StatusBarViewModel {
    fn hotkeys(&self) -> Vec<StatusItem> {
        match self.mode {
            TuiMode::Chat => vec![
                StatusItem { key: "Enter".to_string(), description: "send".to_string() },
                StatusItem { key: "Shift+Enter".to_string(), description: "newline".to_string() },
                StatusItem { key: "^b".to_string(), description: "sidebar".to_string() },
                StatusItem { key: "^k".to_string(), description: "cmd".to_string() },
                StatusItem { key: "?".to_string(), description: "help".to_string() },
                StatusItem { key: "^q".to_string(), description: "quit".to_string() },
            ],
            TuiMode::Overlay => vec![
                StatusItem { key: "Esc".to_string(), description: "close".to_string() },
                StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
                StatusItem { key: "Enter".to_string(), description: "select".to_string() },
            ],
            TuiMode::Select => vec![
                StatusItem { key: "Esc".to_string(), description: "back".to_string() },
                StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
                StatusItem { key: "Enter".to_string(), description: "select".to_string() },
            ],
            TuiMode::Permission => vec![
                StatusItem { key: "y".to_string(), description: "allow".to_string() },
                StatusItem { key: "n".to_string(), description: "deny".to_string() },
                StatusItem { key: "a".to_string(), description: "allow all".to_string() },
            ],
            TuiMode::CommandPalette => vec![
                StatusItem { key: "Esc".to_string(), description: "close".to_string() },
                StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
                StatusItem { key: "Enter".to_string(), description: "run".to_string() },
            ],
            TuiMode::DiffViewer => vec![
                StatusItem { key: "Esc/q/x".to_string(), description: "close".to_string() },
                StatusItem { key: "j/k/↑/↓".to_string(), description: "scroll".to_string() },
                StatusItem { key: "PgUp/PgDn".to_string(), description: "page".to_string() },
            ],
            TuiMode::SessionTree => vec![
                StatusItem { key: "Esc".to_string(), description: "close".to_string() },
                StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
                StatusItem { key: "Enter".to_string(), description: "expand".to_string() },
            ],
            TuiMode::Onboarding => vec![
                StatusItem { key: "Enter".to_string(), description: "next".to_string() },
                StatusItem { key: "Esc".to_string(), description: "back/skip".to_string() },
                StatusItem { key: "^q".to_string(), description: "quit".to_string() },
            ],
        }
    }

    fn center_text(&self) -> Option<String> {
        let model = self.current_model.as_deref()?;
        let tokens = self.session_token_usage.total_tokens;
        let cost = self.session_token_usage.estimated_cost;
        Some(format!("{} │ {} tok │ ${:.4}", model, tokens, cost))
    }

}

impl StatusBar {
    pub fn set_chat_mode(&mut self) {
        self.items = vec![
            StatusItem { key: "Enter".to_string(), description: "send".to_string() },
            StatusItem { key: "^b".to_string(), description: "sidebar".to_string() },
            StatusItem { key: "^k".to_string(), description: "cmd".to_string() },
            StatusItem { key: "^q".to_string(), description: "quit".to_string() },
        ];
    }

    pub fn set_overlay_mode(&mut self) {
        self.items = vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
        ];
    }

    pub fn add_job(&mut self, name: &str) {
        self.background_jobs.push(BackgroundJob {
            name: name.to_string(),
            status: JobStatus::Running,
        });
    }

    pub fn complete_job(&mut self, name: &str) {
        if let Some(job) = self.background_jobs.iter_mut().find(|j| j.name == name) {
            job.status = JobStatus::Complete;
        }
    }

    pub fn clear_completed_jobs(&mut self) {
        self.background_jobs.retain(|j| j.status == JobStatus::Running);
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let sp = StyleHelpers::new(&self.theme);
        let x = area.x + 1;
        let mut current_x = x;
        let mut first = true;

        for item in &self.items {
            if !first {
                let sep_line = Line::from(vec![Span::styled(" | ", sp.tertiary())]);
                buf.set_line(current_x, area.y, &sep_line, 3);
                current_x += 3;
            }
            first = false;

            let parts = vec![
                Span::styled(&item.key, sp.tertiary()),
                Span::raw(" "),
                Span::styled(&item.description, sp.tertiary()),
            ];
            let line = Line::from(parts);
            let item_width = item.key.len() + 1 + item.description.len();
            buf.set_line(current_x, area.y, &line, item_width as u16);
            current_x += item_width as u16;
        }
    }
}

struct StyleHelpers {
    text_tertiary: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
        }
    }
    fn tertiary(&self) -> Style {
        self.text_tertiary
    }
}

pub fn render_ref(vm: &StatusBarViewModel, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let text_tertiary = colors.text_dim;
    let text_secondary = colors.text_secondary;
    let bg = colors.bg_base;

    // Fill background
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg));
            }
        }
    }

    // Left: hotkeys
    let hotkeys = vm.hotkeys();
    let mut x = area.x + 1;
    let mut first = true;

    for item in &hotkeys {
        if !first {
            let sep = Span::styled(" | ", Style::default().fg(text_tertiary));
            let line = Line::from(sep);
            buf.set_line(x, area.y, &line, 3);
            x += 3;
        }
        first = false;

        let parts = vec![
            Span::styled(&item.key, Style::default().fg(text_tertiary)),
            Span::styled(
                format!(" {}", item.description),
                Style::default().fg(text_tertiary).add_modifier(Modifier::DIM),
            ),
        ];
        let line = Line::from(parts);
        let width = (item.key.len() + 1 + item.description.len()) as u16;
        buf.set_line(x, area.y, &line, width);
        x += width;
    }
    let left_end = x;

    // Center: model, tokens, cost
    render_ref_center(area, buf, left_end, text_secondary, vm);
}

/// Renders center text only if it fits without overlapping left side
fn render_ref_center(area: Rect, buf: &mut Buffer, left_end: u16, text_secondary: ratatui::style::Color, vm: &StatusBarViewModel) {
    let Some(center_text) = vm.center_text() else { return };
    let center_width = center_text.chars().count() as u16;
    let min_padding = 2u16;

    let min_center_x = left_end + min_padding;
    let ideal_center_x = area.x + (area.width.saturating_sub(center_width)) / 2;

    let center_x = if ideal_center_x >= min_center_x {
        ideal_center_x
    } else {
        return; // Not enough space on left, skip center
    };

    if center_x + center_width <= area.x + area.width {
        let line = Line::raw(center_text).style(Style::default().fg(text_secondary));
        buf.set_line(center_x, area.y, &line, center_width);
    }
}
