use super::TopBarState;

/// LayoutState contains window/layout related fields.
#[derive(Clone)]
pub struct LayoutState {
    pub show_sidebar: bool,
    pub terminal_size: (u16, u16),
    pub top_bar: TopBarState,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            show_sidebar: false,
            terminal_size: (0, 0),
            top_bar: TopBarState::default(),
        }
    }
}
