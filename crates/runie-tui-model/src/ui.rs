//! Renderer-independent UI actor messages.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMsg {
    HideWelcome,
    ToggleShortcuts,
    ToggleCommandPalette,
    CommandPaletteChar(char),
    CommandPaletteBackspace,
    CommandPaletteMove(isize),
    ActivateCommandPalette,
    Reset,
}
