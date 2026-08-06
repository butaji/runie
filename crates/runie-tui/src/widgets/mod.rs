//! Widgets: scrollback transcript, prompt input, status bar.

pub mod command_palette;
pub mod prompt;
pub mod scrollback;
pub mod status;
pub mod welcome;

pub use command_palette::CommandPaletteWidget;
pub use prompt::{InputMode, PromptOutcome, PromptSnapshot, PromptWidget};
pub use runie_tui_model::PaletteAction;
pub use scrollback::{
    FeedSnapshot, Line, LineKind, Scrollback, ScrollbackMsg, ToolBlock, ToolCardKind,
};
pub use status::{
    braille_spinner_frames, Status, StatusBar, StatusMsg, StatusSnapshot, TurnStatus,
    TurnStatusPhase,
};
pub use welcome::{version_badge, VersionBadgeVariant, WelcomeWidget};
