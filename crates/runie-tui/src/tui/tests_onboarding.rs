#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::tui::state::{AppState, TuiMode, Msg, AnimationState, ContextState, PermissionModalState, CommandPaletteState, ScrollState, ClearInputConfirm, TopBarState};
    use crate::tui::update::update;
    use crate::components::onboarding::Onboarding;
    use crate::components::CommandPalette;
    use runie_ai::TokenUsage;
    use crate::components::SessionTreeNavigator;

    // ─── Onboarding Integration Tests ──────────────────────────────────────────────

    fn make_onboarding_state() -> AppState {
        AppState {
            messages: vec![], textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(), mode: TuiMode::Onboarding,
            running: true, show_sidebar: false, agent_running: false, current_model: None,
            context: ContextState::default(), permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(), scroll: ScrollState::default(),
            animation: AnimationState::default(), diff_viewer: None,
            token_usage: TokenUsage::default(), session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(), background_jobs: Vec::new(),
            onboarding: Some(Onboarding::new(false)), terminal_size: (0, 0),
            clear_input_confirm: ClearInputConfirm::default(),
            model_picker: None, agent_start_time: None,
            input_history: Vec::new(), input_history_index: None,
            input_draft: String::new(), status_header: None,
            status_details: None, status_start_time: None,
            thinking: None, mock_mode: false,
            top_bar: TopBarState::default(),
            last_turn_duration_secs: None, last_turn_tokens: None,
            last_turn_tool_calls: None, turn_success: None,
            slash_menu: crate::components::SlashMenu::new(),
            shortcuts_panel: crate::components::ShortcutsPanel::new(),
            settings_modal: crate::components::SettingsModal::new(),
            home_screen: crate::components::HomeScreen::new(),
            show_thoughts: false, file_picker: crate::components::FilePicker::new(),
            history_search_query: String::new(),
            history_search_matches: Vec::new(),
            history_search_index: 0,
            permission_mode: crate::tui::state::PermissionMode::Normal,
            plan_modal: crate::components::PlanModal::new(),
            allowed_tools: std::collections::HashSet::new(),
            allowed_categories: std::collections::HashSet::new(),
            context_usage_modal: crate::components::ContextUsageModal::new(),
        }
    }

    #[test]
    fn test_onboarding_enter_advances() {
        let mut state = make_onboarding_state();
        let mut palette = CommandPalette::new();
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::Welcome
        );
        update(&mut state, &mut palette, Msg::OnboardingNext);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::ProviderSelect
        );
    }

    #[test]
    fn test_onboarding_esc_goes_back() {
        let mut state = make_onboarding_state();
        let mut palette = CommandPalette::new();
        update(&mut state, &mut palette, Msg::OnboardingNext);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::ProviderSelect
        );
        update(&mut state, &mut palette, Msg::OnboardingBack);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::Welcome
        );
    }

    #[test]
    fn test_onboarding_quit_global() {
        let mut state = make_onboarding_state();
        let mut palette = CommandPalette::new();
        assert!(state.running);
        update(&mut state, &mut palette, Msg::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_onboarding_skip() {
        let mut state = make_onboarding_state();
        let mut palette = CommandPalette::new();
        assert_eq!(state.mode, TuiMode::Onboarding);
        assert!(state.onboarding.is_some());
        update(&mut state, &mut palette, Msg::OnboardingSkip);
        assert!(state.onboarding.is_none());
        assert_eq!(state.mode, TuiMode::Chat);
    }
}
