//! `runie-tui` — minimal ratatui/crossterm TUI that subscribes to
//! `runie-core`'s event stream and renders a 3-region chat interface
//! (scrollback transcript + prompt input + status bar).
//!
//! See `tasks/` for the implementation plan; the e2e test
//! (`tests/e2e_test.rs`) is the behavioural contract.

pub mod app;
pub mod appearance;
pub mod event_renderer;
pub mod key;
pub mod layout;
pub mod scrollback_actor;
pub mod status_actor;
pub mod widgets;
pub mod yaml_runner;

pub use app::{App, AppExit};
pub use event_renderer::EventRenderer;
pub use key::{map_key, Action};
pub use layout::{
    chat_layout, chat_layout_with_prompt_height, ChatLayout, PROMPT_HEIGHT, STATUS_HEIGHT,
};
pub use scrollback_actor::ScrollbackActor;
pub use status_actor::StatusActor;
pub use widgets::{
    version_badge, InputMode, PromptOutcome, PromptWidget, Scrollback, ScrollbackMsg, Status,
    StatusBar, StatusMsg, TurnStatus, TurnStatusPhase, VersionBadgeVariant,
};
