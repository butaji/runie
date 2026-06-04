//! StatusBar component — kept for `BackgroundJob` and `JobStatus` types
//! used by `AppState.background_jobs` and the activity panel.
//!
//! The legacy `StatusBar` struct + `Widget` impl were removed in H2 of the
//! UI architecture review; the live render path goes through
//! `StatusBarViewModel::hotkeys()`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use crate::theme::ThemeColors;
use crate::tui::state::TuiMode;
use crate::tui::view_models::{McpStatus, StatusBarViewModel};

pub mod builder;
mod render;
#[cfg(test)]
mod mod_test;
pub use builder::*;

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

impl StatusBarViewModel {
    /// Returns context-aware hotkeys based on current state.
    /// When agent_running=true, shows 4 hints (including Ctrl+c, Ctrl+Enter).
    /// Otherwise shows mode-specific hints (typically 2 for idle chat).
    pub fn hotkeys(&self) -> Vec<StatusItem> {
        // Context-aware: agent running > input has text > mode-specific
        let result = if self.agent_running {
            Self::agent_running_hotkeys()
        } else if self.input_has_text {
            Self::input_with_text_hotkeys()
        } else {
            hotkeys_for_mode(self.mode)
        };
        // DEBUG: trace hotkeys selection
        tracing::debug!(
            "hotkeys() called: agent_running={}, input_has_text={}, mode={:?} -> returning {} items",
            self.agent_running,
            self.input_has_text,
            self.mode,
            result.len()
        );
        result
    }

    /// Hotkeys shown when agent is running
    pub fn agent_running_hotkeys() -> Vec<StatusItem> {
        // Grok-style hints for agent running state
        vec![
            StatusItem { key: "Shift+Tab".to_string(), description: "mode".to_string() },
            StatusItem { key: "Ctrl+c".to_string(), description: "cancel".to_string() },
            StatusItem { key: "Ctrl+Enter".to_string(), description: "interject".to_string() },
            StatusItem { key: "Ctrl+.".to_string(), description: "shortcuts".to_string() },
        ]
    }

    /// Hotkeys shown when input has text and agent is idle
    fn input_with_text_hotkeys() -> Vec<StatusItem> {
        // Grok-style minimal hints
        vec![
            StatusItem { key: "Shift+Tab".to_string(), description: "mode".to_string() },
            StatusItem { key: "Ctrl+.".to_string(), description: "shortcuts".to_string() },
        ]
    }

    fn center_text(&self) -> Option<String> {
        let model = self.current_model.as_deref()?;
        let tokens = self.session_token_usage.total_tokens;
        let cost = self.session_token_usage.estimated_cost;
        if cost > 0.0 {
            Some(format!("{} · {} tok · ${:.4}", model, tokens, cost))
        } else {
            Some(format!("{} · {} tok", model, tokens))
        }
    }

    /// Hotkeys shown in idle state (when agent is not running and input is empty).
    /// Alias for chat_hotkeys - same 2 hints (Shift+Tab, Ctrl+.)
    pub fn idle_hotkeys() -> Vec<StatusItem> {
        Self::chat_hotkeys()
    }

    fn chat_hotkeys() -> Vec<StatusItem> {
        // Grok-style minimal hints: only mode toggle and shortcuts
        vec![
            StatusItem { key: "Shift+Tab".to_string(), description: "mode".to_string() },
            StatusItem { key: "Ctrl+.".to_string(), description: "shortcuts".to_string() },
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
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
            StatusItem { key: "Backspace".to_string(), description: "delete".to_string() },
        ]
    }

    fn diff_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k".to_string(), description: "scroll".to_string() },
        ]
    }

    fn tree_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
        ]
    }

    fn onboarding_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Enter".to_string(), description: "next".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Esc".to_string(), description: "back".to_string() },
        ]
    }

    #[allow(dead_code)]
    fn home_hotkeys() -> Vec<StatusItem> {
        Self::tree_hotkeys()
    }

    #[allow(dead_code)]
    fn plan_hotkeys() -> Vec<StatusItem> {
        Self::tree_hotkeys()
    }

    fn fullscreen_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "j/k".to_string(), description: "scroll".to_string() },
        ]
    }

    fn default_hotkeys() -> Vec<StatusItem> {
        vec![
            StatusItem { key: "Esc".to_string(), description: "close".to_string() },
            StatusItem { key: "↑/↓".to_string(), description: "navigate".to_string() },
            StatusItem { key: "Enter".to_string(), description: "select".to_string() },
        ]
    }
}

fn hotkeys_for_mode(mode: TuiMode) -> Vec<StatusItem> {
    match mode {
        TuiMode::Chat | TuiMode::Subagents | TuiMode::Questionnaire => StatusBarViewModel::chat_hotkeys(),
        TuiMode::Overlay => StatusBarViewModel::overlay_hotkeys(),
        TuiMode::Select => StatusBarViewModel::select_hotkeys(),
        TuiMode::Permission => StatusBarViewModel::permission_hotkeys(),
        TuiMode::CommandPalette => StatusBarViewModel::palette_hotkeys(),
        TuiMode::DiffViewer | TuiMode::FullscreenViewer => StatusBarViewModel::diff_hotkeys(),
        TuiMode::SessionTree | TuiMode::Onboarding | TuiMode::Plan | TuiMode::HomeScreen => StatusBarViewModel::tree_hotkeys(),
    }
}

pub use render::render_ref;

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
            agent_running: false,
            input_has_text: false,
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
            agent_running: false,
            input_has_text: false,
        }
    }

    fn theme_colors() -> ThemeColors {
        use ratatui::style::Color;
        ThemeColors {
            bg_base: Color::Reset, bg_panel: Color::Black, text_primary: Color::White,
            text_secondary: Color::Gray, text_dim: Color::DarkGray, text_muted: Color::DarkGray,
            accent_primary: Color::Blue, accent_secondary: Color::Cyan,
            border_unfocused: Color::DarkGray, success: Color::Green, error: Color::Red,
            warning: Color::Yellow, syntax_phase: Color::Yellow, text_plan: Color::Magenta,
            feed_tool_bar: Color::LightBlue, accent_user: Color::Blue, accent_assistant: Color::Cyan,
            accent_thinking: Color::Yellow, accent_tool: Color::Magenta,
            accent_system: Color::DarkGray, accent_error: Color::Red, accent_success: Color::Green,
            accent_running: Color::Yellow, accent_skill: Color::Blue, accent_plan: Color::Yellow,
            accent_feedback: Color::Red, accent_model: Color::Cyan, accent_teal: Color::Cyan,
            accent_orange: Color::Yellow, accent_purple: Color::Magenta, accent_yellow: Color::Yellow,
            accent_blue_bright: Color::Blue, command: Color::Blue, path: Color::Cyan,
            running: Color::Yellow, fuzzy_accent: Color::Blue, editor_bg: Color::Black,
            surface_bg: Color::Black, popover_bg: Color::Black,
        }
    }

    fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
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
        let area = Rect::new(0, 0, 120, 1);
        let mut buf = Buffer::empty(area);
        let colors = theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

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

        assert!(buffer_contains(&buf, "Enter"),
            "Onboarding mode should display Enter hotkey");
        assert!(buffer_contains(&buf, "Esc"),
            "Onboarding mode should display Esc hotkey");
    }
}
