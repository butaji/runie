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

    fn make_theme_colors() -> ThemeColors {
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
        let colors = make_theme_colors();

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
        let colors = make_theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

        assert!(buffer_contains(&buf, "Enter"),
            "Chat mode should display Enter hotkey");
    }

    #[test]
    fn test_onboarding_mode_shows_hotkeys() {
        let vm = make_onboarding_vm_with_model();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let colors = make_theme_colors();

        render_ref(&vm, area, &mut buf, &colors);

        assert!(buffer_contains(&buf, "Enter"),
            "Onboarding mode should display Enter hotkey");
        assert!(buffer_contains(&buf, "Esc"),
            "Onboarding mode should display Esc hotkey");
    }
}
