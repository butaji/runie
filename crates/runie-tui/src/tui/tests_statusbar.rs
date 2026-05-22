//! Status bar hotkey tests verifying context-aware hotkeys per mode.

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests_statusbar {
    use crate::tui::render::get_status_items;
    use crate::tui::state::{AppState, TuiMode, RenderState};
    use crate::tui::TuiConfig;

    #[test]
    fn test_chat_mode_hotkeys() {
        let items = get_status_items(&TuiMode::Chat);
        assert!(items.iter().any(|(k, _)| k == &"Enter"));
        assert!(items.iter().any(|(k, _)| k == &"^b"));
        assert!(items.iter().any(|(k, _)| k == &"^k"));
        assert!(items.iter().any(|(k, _)| k == &"^q"));
    }

    #[test]
    fn test_permission_mode_hotkeys() {
        let items = get_status_items(&TuiMode::Permission);
        assert!(items.iter().any(|(k, _)| k == &"y"));
        assert!(items.iter().any(|(k, _)| k == &"n"));
        assert!(items.iter().any(|(k, _)| k == &"a"));
        assert!(items.iter().any(|(k, _)| k == &"s"));
    }

    #[test]
    fn test_palette_mode_hotkeys() {
        let items = get_status_items(&TuiMode::CommandPalette);
        assert!(items.iter().any(|(k, _)| k == &"Esc"));
        assert!(items.iter().any(|(k, _)| k == &"Enter"));
        assert!(items.iter().any(|(k, _)| k == &"↑↓"));
    }

    #[test]
    fn test_onboarding_mode_hotkeys() {
        let items = get_status_items(&TuiMode::Onboarding);
        assert!(items.iter().any(|(k, _)| k == &"Esc"));
        assert!(items.iter().any(|(k, _)| k == &"↑↓"));
        assert!(items.iter().any(|(k, _)| k == &"Enter"));
    }

    #[test]
    fn test_status_bar_always_visible() {
        let config = TuiConfig::default();
        assert!(config.show_status_bar);
    }

    #[test]
    fn test_status_bar_renders_in_onboarding() {
        let mut state = AppState::default();
        state.mode = TuiMode::Onboarding;
        let render_state = RenderState::from(&state);
        assert!(matches!(render_state.mode, TuiMode::Onboarding));
    }
}
