//! Widgets: scrollback transcript, prompt input, status bar.

pub mod prompt;
pub mod scrollback;
pub mod status;
pub mod welcome;

pub use prompt::{PromptOutcome, PromptWidget};
pub use scrollback::{Line, LineKind, Scrollback};
pub use status::{Status, StatusBar, TurnStatus, TurnStatusPhase};
pub use welcome::WelcomeWidget;
