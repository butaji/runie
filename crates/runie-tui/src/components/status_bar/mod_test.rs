#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::view_models::StatusBarViewModel;

    #[test]
    fn test_idle_hotkeys_count() {
        let hints = StatusBarViewModel::idle_hotkeys();
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_agent_running_hotkeys_count() {
        let hints = StatusBarViewModel::agent_running_hotkeys();
        assert_eq!(hints.len(), 4);
    }

    #[test]
    fn test_idle_hotkeys_content() {
        let hints = StatusBarViewModel::idle_hotkeys();
        assert_eq!(hints[0].key, "Shift+Tab");
        assert_eq!(hints[0].description, "mode");
        assert_eq!(hints[1].key, "Ctrl+.");
        assert_eq!(hints[1].description, "shortcuts");
    }

    #[test]
    fn test_agent_running_hotkeys_content() {
        let hints = StatusBarViewModel::agent_running_hotkeys();
        assert_eq!(hints[0].key, "Shift+Tab");
        assert_eq!(hints[1].key, "Ctrl+c");
        assert_eq!(hints[1].description, "cancel");
        assert_eq!(hints[2].key, "Ctrl+Enter");
        assert_eq!(hints[2].description, "interject");
        assert_eq!(hints[3].key, "Ctrl+.");
    }

    #[test]
    fn test_status_bar_vm_uses_correct_hints() {
        let mut vm = StatusBarViewModel::default();
        vm.agent_running = false;
        assert_eq!(vm.hotkeys().len(), 2);

        vm.agent_running = true;
        assert_eq!(vm.hotkeys().len(), 4);
    }
}
