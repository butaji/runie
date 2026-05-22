use crate::tui::state::{AppState, TuiMode, Msg, AnimationState, TopBarState, PermissionModalState, CommandPaletteState, ScrollState};
use crate::tui::update::update;
use crate::components::onboarding::Onboarding;
use runie_ai::TokenUsage;
use crate::components::SessionTreeNavigator;


#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    // ─── Onboarding Integration Tests ──────────────────────────────────────────────

    fn make_onboarding_state() -> AppState {
        AppState {
            messages: vec![],
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Onboarding,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar: TopBarState::default(),
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            scroll: ScrollState::default(),
            animation: AnimationState::default(),
            diff_viewer: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            session_tree: SessionTreeNavigator::new(),
            background_jobs: Vec::new(),
            onboarding: Some(Onboarding::new()),
        }
    }

    #[test]
    fn test_onboarding_enter_advances() {
        let mut state = make_onboarding_state();
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::Welcome
        );
        update(&mut state, Msg::OnboardingNext);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::ProviderSelect
        );
    }

    #[test]
    fn test_onboarding_esc_goes_back() {
        let mut state = make_onboarding_state();
        update(&mut state, Msg::OnboardingNext);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::ProviderSelect
        );
        update(&mut state, Msg::OnboardingBack);
        assert_eq!(
            state.onboarding.as_ref().unwrap().step,
            crate::components::onboarding::OnboardingStep::Welcome
        );
    }

    #[test]
    fn test_onboarding_quit_global() {
        let mut state = make_onboarding_state();
        assert!(state.running);
        update(&mut state, Msg::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_onboarding_skip() {
        let mut state = make_onboarding_state();
        assert_eq!(state.mode, TuiMode::Onboarding);
        assert!(state.onboarding.is_some());
        update(&mut state, Msg::OnboardingSkip);
        assert!(state.onboarding.is_none());
        assert_eq!(state.mode, TuiMode::Chat);
    }
}
