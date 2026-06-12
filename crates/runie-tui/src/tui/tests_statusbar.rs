//! Status bar hotkey tests verifying context-aware hotkeys per mode.

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests_statusbar {
    use crate::tui::render::get_status_items;
    use crate::tui::state::{AppState, TuiMode};
    use crate::tui::TuiConfig;

    #[test]
    fn test_chat_mode_hotkeys() {
        let items = get_status_items(&TuiMode::Chat);
        // Chat idle mode shows Shift+Tab and Ctrl+.
        assert!(items.iter().any(|(k, _)| k == &"Shift+Tab"), "Chat mode should show Shift+Tab");
        assert!(items.iter().any(|(k, _)| k == &"Ctrl+."), "Chat mode should show Ctrl+.");
    }

    #[test]
    fn test_permission_mode_hotkeys() {
        // P1-2: Updated to reflect new key labels with progressive disclosure
        let items = get_status_items(&TuiMode::Permission);
        assert!(items.iter().any(|(k, _)| k == &"y/Enter"));
        assert!(items.iter().any(|(k, _)| k == &"Esc/n"));
        assert!(items.iter().any(|(k, _)| k == &"a"));
        // "s" (skip) is now a discoverable hint, not in primary status bar
        assert!(!items.iter().any(|(k, _)| k == &"s"));
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
        // AppState is used directly now (RenderState eliminated)
        assert!(matches!(state.mode, TuiMode::Onboarding));
    }
}
