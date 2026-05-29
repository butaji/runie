use super::TuiMode;

/// UiModeState wraps the TuiMode enum for explicit mode state management.
#[derive(Clone)]
pub struct UiModeState {
    pub mode: TuiMode,
}

impl Default for UiModeState {
    fn default() -> Self {
        Self {
            mode: TuiMode::Chat,
        }
    }
}
