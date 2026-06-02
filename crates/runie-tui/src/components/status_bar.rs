use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::{ThemeColors, ThemeWrapper};
use crate::tui::state::TuiMode;
use crate::tui::view_models::{McpStatus, StatusBarViewModel};

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
    pub progress: f64, // 0.0 to 1.0
}

impl Default for BackgroundJob {
    fn default() -> Self {
        Self {
            name: String::new(),
            status: JobStatus::Running,
            progress: 0.0,
        }
    }
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
            TuiMode::Chat => Self::chat_hotkeys(),
            TuiMode::Overlay => Self::overlay_hotkeys(),
            TuiMode::Select => Self::select_hotkeys(),
            TuiMode::Permission => Self::permission_hotkeys(),
            TuiMode::CommandPalette => Self::palette_hotkeys(),
            TuiMode::DiffViewer => Self::diff_hotkeys(),
            TuiMode::SessionTree => Self::tree_hotkeys(),
            TuiMode::Onboarding => Self::onboarding_hotkeys(),
            TuiMode::HomeScreen => Self::home_hotkeys(),
            TuiMode::Plan => Self::plan_hotkeys(),
            TuiMode::Subagents => Self::chat_hotkeys(),
            TuiMode::Questionnaire => Self::chat_hotkeys(),
            TuiMode::FullscreenViewer => Self::fullscreen_hotkeys(),
        }
    }

    fn chat_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Enter".to_string(), description: "send".to_string() },
            StatusItem { key: "Shift+Enter".to_string(), description: "newline".to_string() },
            StatusItem { key: "^b".to_string(), description: "sidebar".to_string() },
            StatusItem { key: "^k".to_string(), description: "cmd".to_string() },
            StatusItem { key: "?".to_string(), description: "help".to_string() },
            StatusItem { key: "^q".to_string(), description: "quit".to_string() },
        ]
    }

    fn overlay_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
        ]
    }

    fn select_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "back".to_string() },
            StatusItem { key: "j/k".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
        ]
    }

    fn permission_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "y".to_string(), description: "allow".to_string() },
            StatusItem { key: "n".to_string(), description: "deny".to_string() },
            StatusItem { key: "a".to_string(), description: "allow all".to_string() },
        ]
    }

    fn palette_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "run".to_string() },
        ]
    }

    fn diff_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc/q/x".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k/↑/↓".to_string(), description: "scroll".to_string() },
            StatusItem { key: "PgUp/PgDn".to_string(), description: "page".to_string() },
        ]
    }

    fn tree_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "expand".to_string() },
        ]
    }

    fn home_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
            StatusItem { key: "q".to_string(), description: "quit".to_string() },
        ]
    }

    fn plan_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Enter".to_string(), description: "approve".to_string() },
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "scroll".to_string() },
        ]
    }

    fn onboarding_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Enter".to_string(), description: "next".to_string() },
            StatusItem { key: "Esc".to_string(), description: "back/skip".to_string() },
            StatusItem { key: "^q".to_string(), description: "quit".to_string() },
        ]
    }

    fn fullscreen_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "q/Esc/Enter".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k".to_string(), description: "scroll".to_string() },
            StatusItem { key: "g/G".to_string(), description: "top/bottom".to_string() },
        ]
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
            progress: 0.0,
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

    fill_status_background(area, buf, bg);

    let hotkeys = vm.hotkeys();
    let left_end = render_hotkey_items(area, buf, &hotkeys, text_tertiary);

    // During onboarding, only show hotkeys - hide model/token/cost info
    if !matches!(vm.mode, TuiMode::Onboarding) {
        render_ref_center(area, buf, left_end, text_secondary, vm);
    }

    // Render MCP status on the right side
    render_mcp_status(&vm.mcp_status, area, buf, colors);
}

fn fill_status_background(area: Rect, buf: &mut Buffer, bg: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_style(Style::default().bg(bg));
            }
        }
    }
}

fn render_hotkey_items(area: Rect, buf: &mut Buffer, hotkeys: &[StatusItem], text_tertiary: ratatui::style::Color) -> u16 {
    let mut x = area.x + 1;
    let mut first = true;

    for item in hotkeys {
        if !first {
            let sep = Span::styled(" | ", Style::default().fg(text_tertiary));
            let line = Line::from(sep);
            buf.set_line(x, area.y, &line, 3);
            x += 3;
        }
        first = false;

        let parts = vec![
            Span::styled(&item.key, Style::default().fg(text_tertiary)),
            Span::styled(format!(" {}", item.description), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)),
        ];
        let line = Line::from(parts);
        let width = (item.key.len() + 1 + item.description.len()) as u16;
        buf.set_line(x, area.y, &line, width);
        x += width;
    }
    x
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

fn render_mcp_status(mcp: &McpStatus, area: Rect, buf: &mut Buffer, colors: &ThemeColors) {
    let text = match mcp {
        McpStatus::Connected(n) if *n > 0 => {
            format!("⚡ {} MCP servers", n)
        }
        McpStatus::Unavailable(n) if *n > 0 => {
            format!("⛔ {} MCP servers unavailable", n)
        }
        _ => return,
    };

    let line_width = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(line_width + 1);
    let line = Line::styled(text, Style::default().fg(colors.text_dim));
    buf.set_line(x, area.y, &line, line_width);
}

#[cfg(test)]
mod tests_status_bar_onboarding {
    use super::*;
    use runie_ai::TokenUsage;

    fn make_onboarding_vm_with_model() -> StatusBarViewModel {
        StatusBarViewModel {
            mode: TuiMode::Onboarding,
            current_model: Some("openai/gpt-4o".to_string()),
            session_token_usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                estimated_cost: 0.0,
            },
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: McpStatus::None,
        }
    }

    fn make_chat_vm_with_model() -> StatusBarViewModel {
        StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("openai/gpt-4o".to_string()),
            session_token_usage: TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
                estimated_cost: 0.0023,
            },
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: McpStatus::None,
        }
    }

    fn theme_colors() -> ThemeColors {
        ThemeColors {
            bg_base: ratatui::style::Color::Reset,
            bg_panel: ratatui::style::Color::Black,
            text_primary: ratatui::style::Color::White,
            text_secondary: ratatui::style::Color::Gray,
            text_dim: ratatui::style::Color::DarkGray,
            text_muted: ratatui::style::Color::DarkGray,
            accent_primary: ratatui::style::Color::Blue,
            accent_secondary: ratatui::style::Color::Cyan,
            border_unfocused: ratatui::style::Color::DarkGray,
            success: ratatui::style::Color::Green,
            error: ratatui::style::Color::Red,
            warning: ratatui::style::Color::Yellow,
            syntax_phase: ratatui::style::Color::Yellow,
            text_plan: ratatui::style::Color::Magenta,
            feed_tool_bar: ratatui::style::Color::LightBlue,
            accent_user: ratatui::style::Color::Blue,
            accent_assistant: ratatui::style::Color::Cyan,
            accent_thinking: ratatui::style::Color::Yellow,
            accent_tool: ratatui::style::Color::Magenta,
            accent_system: ratatui::style::Color::DarkGray,
            accent_error: ratatui::style::Color::Red,
            accent_success: ratatui::style::Color::Green,
            accent_running: ratatui::style::Color::Yellow,
            accent_skill: ratatui::style::Color::Blue,
            accent_plan: ratatui::style::Color::Yellow,
            accent_feedback: ratatui::style::Color::Red,
            accent_model: ratatui::style::Color::Cyan,
            accent_teal: ratatui::style::Color::Cyan,
            accent_orange: ratatui::style::Color::Yellow,
            accent_purple: ratatui::style::Color::Magenta,
            accent_yellow: ratatui::style::Color::Yellow,
            accent_blue_bright: ratatui::style::Color::Blue,
            command: ratatui::style::Color::Blue,
            path: ratatui::style::Color::Cyan,
            running: ratatui::style::Color::Yellow,
            fuzzy_accent: ratatui::style::Color::Blue,
            editor_bg: ratatui::style::Color::Black,
            surface_bg: ratatui::style::Color::Black,
            popover_bg: ratatui::style::Color::Black,
        }
    }

    fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
        // Collect each line as a string and check if it contains the text
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains(text) {
                return true;
            }
        }
        false
    }

    #[test]
    fn test_onboarding_mode_hides_model_info() {
        let vm = make_onboarding_vm_with_model();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let colors = theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

        // Onboarding mode should NOT show model/token/cost info
        assert!(!buffer_contains(&buf, "openai/gpt-4o"),
            "Onboarding mode should not display model name");
        assert!(!buffer_contains(&buf, "tok"),
            "Onboarding mode should not display token count");
        assert!(!buffer_contains(&buf, "$"),
            "Onboarding mode should not display cost");
    }

    #[test]
    fn test_chat_mode_shows_hotkeys() {
        let vm = make_chat_vm_with_model();
        let area = Rect::new(0, 0, 120, 1);  // Wider area to fit hotkeys + center
        let mut buf = Buffer::empty(area);
        let colors = theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

        // Chat mode should show hotkeys (model info moved to GlobalTags)
        assert!(buffer_contains(&buf, "Enter"),
            "Chat mode should display Enter hotkey");
    }

    #[test]
    fn test_onboarding_mode_shows_hotkeys() {
        let vm = make_onboarding_vm_with_model();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let colors = theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

        // Onboarding mode SHOULD show hotkeys
        assert!(buffer_contains(&buf, "Enter"),
            "Onboarding mode should display Enter hotkey");
        assert!(buffer_contains(&buf, "Esc"),
            "Onboarding mode should display Esc hotkey");
    }
}
