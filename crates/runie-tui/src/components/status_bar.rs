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
                StatusItem { key: "^b".to_string(), description: "sidebar".to_string() },
                StatusItem { key: "^k".to_string(), description: "cmd".to_string() },
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
                StatusItem { key: "Esc".to_string(), description: "close".to_string() },
                StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
                StatusItem { key: "y".to_string(), description: "accept".to_string() },
                StatusItem { key: "n".to_string(), description: "reject".to_string() },
            ],
            TuiMode::SessionTree => vec![
                StatusItem { key: "Esc".to_string(), description: "close".to_string() },
                StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
                StatusItem { key: "Enter".to_string(), description: "expand".to_string() },
            ],
            TuiMode::Onboarding => vec![
                StatusItem { key: "Enter".to_string(), description: "next".to_string() },
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

    fn right_text(&self) -> Option<String> {
        let running_jobs: Vec<_> = self
            .background_jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .collect();
        if running_jobs.is_empty() {
            return None;
        }
        let job_count = running_jobs.len();
        let latest_job = running_jobs.last()?;
        let braille = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let spinner = braille[self.braille_frame % 10];
        let text = if job_count == 1 {
            format!("⬡ {} │ {} {}", latest_job.name, spinner, latest_job.name)
        } else {
            format!("⬡ {} jobs │ {} {}", job_count, spinner, latest_job.name)
        };
        Some(text)
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

    // Right: background jobs
    if let Some(jobs_text) = vm.right_text() {
        let jobs_width = jobs_text.chars().count() as u16;
        let jobs_x = area.x + area.width - jobs_width - 1;
        let line = Line::raw(jobs_text).style(Style::default().fg(text_secondary));
        buf.set_line(jobs_x, area.y, &line, jobs_width);
    }

    // Center: model, tokens, cost
    if let Some(center_text) = vm.center_text() {
        let center_width = center_text.chars().count() as u16;
        let center_x = area.x + (area.width.saturating_sub(center_width)) / 2;
        let line = Line::raw(center_text).style(Style::default().fg(text_secondary));
        buf.set_line(center_x, area.y, &line, center_width);
    }
}
