//! Widgets: scrollback transcript, prompt input, status bar.

pub mod dialog;
pub mod prompt;
pub mod scrollback;
pub mod status;
pub mod welcome;

pub use dialog::DialogWidget;
pub use prompt::{InputMode, PromptOutcome, PromptSnapshot, PromptWidget};
pub use runie_tui_model::PaletteAction;
pub use runie_tui_model::TuiSnapshot;
pub use runie_tui_model::{CellPosition, CellSelection};
pub use scrollback::{
    FeedSnapshot, Line, LineKind, Scrollback, ScrollbackMsg, ToolBlock, ToolCardKind,
};
pub use status::{
    braille_spinner_frames, Status, StatusBar, StatusMsg, StatusSnapshot, TurnStatus,
    TurnStatusPhase,
};
pub use welcome::{version_badge, welcome_modal_lines, VersionBadgeVariant, WelcomeWidget};
